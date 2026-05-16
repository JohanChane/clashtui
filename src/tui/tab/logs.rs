use super::dev::*;
use crate::config::CONFIG;
use crate::functions::restful::api_log::{self, LogEntry};
use crate::functions::restful::config;
use ratatui::text::Line;
use ratatui::widgets::{List, ListItem};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

newtype_tab!(LogsTab(Tab<Logs>));

mod_agent!(
    Key,
    [
        ([KeyCode::Up], Key::MoveUp, "Move up"),
        ([KeyCode::Down], Key::MoveDown, "Move down"),
        ([KeyCode::Char('k')], Key::MoveUp, "Move up"),
        ([KeyCode::Char('j')], Key::MoveDown, "Move down"),
        ([KeyCode::Char('G')], Key::GoBottom, "Go to bottom"),
        (
            [KeyCode::Char('g'), KeyCode::Char('g')],
            Key::GoTop,
            "Go to top"
        ),
        ([KeyCode::Char('/')], Key::Search, "Search/Filter"),
        ([KeyCode::Char('p')], Key::TogglePause, "Pause/Resume"),
        ([KeyCode::Char('f')], Key::FzfFind, "Find"),
        ([KeyCode::Char('c')], Key::Clear, "Clear logs"),
        (
            [KeyCode::Char('t'), KeyCode::Char('d')],
            Key::ToggleDebug,
            "Toggle debug"
        ),
        (
            [KeyCode::Char('t'), KeyCode::Char('i')],
            Key::ToggleInfo,
            "Toggle info"
        ),
        (
            [KeyCode::Char('t'), KeyCode::Char('w')],
            Key::ToggleWarning,
            "Toggle warning"
        ),
        (
            [KeyCode::Char('t'), KeyCode::Char('e')],
            Key::ToggleError,
            "Toggle error"
        ),
        (
            [KeyCode::Char('t'), KeyCode::Char('s')],
            Key::ToggleSilent,
            "Toggle silent"
        ),
    ]
);

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum Key {
    MoveUp,
    MoveDown,
    GoTop,
    GoBottom,
    Search,
    TogglePause,
    FzfFind,
    Clear,
    ToggleDebug,
    ToggleInfo,
    ToggleWarning,
    ToggleError,
    ToggleSilent,
}

impl TryFrom<&crate::tui::Key> for Key {
    type Error = ();

    fn try_from(ev: &crate::tui::Key) -> Result<Self, Self::Error> {
        let agent = agent();
        if !agent.is_empty() {
            return agent.get(ev).map(|act| *act).ok_or(());
        }
        Err(())
    }
}

const LOG_BUFFER_SIZE: usize = 300;

struct LogBuffer {
    entries: [Option<LogEntry>; LOG_BUFFER_SIZE],
    tail: isize,
    len: usize,
}

impl LogBuffer {
    fn new() -> Self {
        const NONE: Option<LogEntry> = None;
        Self {
            entries: [NONE; LOG_BUFFER_SIZE],
            tail: -1,
            len: 0,
        }
    }

    fn push(&mut self, entry: LogEntry) {
        self.tail = (self.tail + 1) % LOG_BUFFER_SIZE as isize;
        self.entries[self.tail as usize] = Some(entry);
        if self.len < LOG_BUFFER_SIZE {
            self.len += 1;
        }
    }

    fn clear(&mut self) {
        self.tail = -1;
        self.len = 0;
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn count(&self) -> usize {
        self.len
    }

    fn iter_from_head(&self) -> impl Iterator<Item = &LogEntry> {
        let start = if self.len < LOG_BUFFER_SIZE {
            0
        } else {
            ((self.tail + 1) % LOG_BUFFER_SIZE as isize) as usize
        };
        let count = self.len;
        (0..count).filter_map(move |i| {
            let idx = (start + i) % LOG_BUFFER_SIZE;
            self.entries[idx].as_ref()
        })
    }
}

impl Default for Logs {
    fn default() -> Self {
        Self {
            buffer: LogBuffer::new(),
            scroll: 0,
            error: None,
            filter: None,
            paused: true,
            current_log_level: String::new(),
            ws_pending: None,
            ws_level: Arc::new(Mutex::new(String::new())),
            ws_reconnect: Arc::new(AtomicBool::new(false)),
        }
    }
}

struct Logs {
    buffer: LogBuffer,
    scroll: usize,
    error: Option<String>,
    filter: Option<String>,
    paused: bool,
    current_log_level: String,
    ws_pending: Option<Arc<Mutex<Vec<LogEntry>>>>,
    ws_level: Arc<Mutex<String>>,
    ws_reconnect: Arc<AtomicBool>,
}

fn spawn_ws_logs(
    controller: String,
    secret: Option<String>,
    pending: Arc<Mutex<Vec<LogEntry>>>,
    level: Arc<Mutex<String>>,
    reconnect: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        let ws_scheme = if controller.starts_with("https") {
            "wss"
        } else {
            "ws"
        };
        // Strip http(s):// prefix and trailing slash if any
        let addr = controller
            .strip_prefix("http://")
            .or_else(|| controller.strip_prefix("https://"))
            .unwrap_or(&controller)
            .trim_end_matches('/');

        loop {
            let current_level = level.lock().unwrap().clone();
            reconnect.store(false, Ordering::Relaxed);

            let url_str = if let Some(ref s) = secret {
                format!("{ws_scheme}://{addr}/logs?token={s}&level={current_level}")
            } else {
                format!("{ws_scheme}://{addr}/logs?level={current_level}")
            };

            match tungstenite::connect(&url_str) {
                Ok((mut ws, _)) => {
                    // Set read timeout on inner TcpStream for periodic reconnect checks
                    if let tungstenite::stream::MaybeTlsStream::Plain(stream) = ws.get_mut() {
                        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                    }

                    loop {
                        match ws.read() {
                            Ok(tungstenite::Message::Text(text)) => {
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                                    let type_ = v
                                        .get("type")
                                        .and_then(|t| t.as_str())
                                        .unwrap_or("unknown")
                                        .to_owned();
                                    let payload = v
                                        .get("payload")
                                        .and_then(|p| p.as_str())
                                        .unwrap_or("")
                                        .to_owned();
                                    pending.lock().unwrap().push(LogEntry {
                                        type_,
                                        payload,
                                        time: api_log::timestamp(),
                                    });
                                }
                            }
                            Ok(tungstenite::Message::Close(_)) => break,
                            Err(tungstenite::Error::Io(ref e))
                                if e.kind() == std::io::ErrorKind::WouldBlock
                                    || e.kind() == std::io::ErrorKind::TimedOut =>
                            {
                                if reconnect.load(Ordering::Relaxed) {
                                    break;
                                }
                                continue;
                            }
                            Err(e) => {
                                log::warn!("WebSocket read error: {e}");
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    log::warn!("WebSocket connect error: {e}");
                }
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    });
}

impl BasicTabContent for Logs {
    type Key = Key;
    type State = ();

    const TITLE: &str = "Logs";

    fn all_shortcuts() -> &'static [(KeyCombo, Self::Key, &'static str)] {
        agent::all_shortcuts()
    }

    fn on_enter(&mut self, task_set: &mut FutureSet<Self>, _state: &mut Self::State) {
        if crate::config::is_core_mismatch() {
            self.buffer.clear();
            self.error = Some("API data mismatch with configured core".to_owned());
            self.paused = true;
            return;
        }
        // Refresh log level from core on every re-entry
        async {
            let cfg = tri!(
                tokio::task::spawn_blocking(config::fetch).await.unwrap(),
                or_set
            );
            wrapper(move |content: &mut Self| {
                if crate::config::is_core_mismatch() {
                    return;
                }
                let level = cfg
                    .log_level
                    .as_ref()
                    .map(|l| l.to_string())
                    .unwrap_or_else(|| "unknown".to_owned());
                content.current_log_level = level.clone();
                *content.ws_level.lock().unwrap() = level;
            })
        }
        .spawn_at(task_set);
    }

    fn after_sync(&self, task_set: &mut FutureSet<Self>) {
        if self.paused {
            return;
        }
        if crate::config::is_core_mismatch() {
            return;
        }
        if let Some(ref pending) = self.ws_pending {
            let pending = Arc::clone(pending);
            async move {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                let entries: Vec<LogEntry> = pending.lock().unwrap().drain(..).collect();
                wrapper(move |content: &mut Self| {
                    for entry in entries {
                        content.buffer.push(entry);
                    }
                    if content.buffer.count() > 0
                        && content.scroll + 1 >= content.buffer.count().saturating_sub(1)
                    {
                        content.scroll = content.buffer.count().saturating_sub(1);
                    }
                })
            }
            .spawn_at(task_set);
        }
    }
}

impl TabContent for Logs {
    fn init(&mut self, task_set: &mut FutureSet<Self>, _state: &mut Self::State) {
        let pending = Arc::new(Mutex::new(Vec::new()));
        self.ws_pending = Some(Arc::clone(&pending));
        let controller = CONFIG.controller_for_core().to_owned();
        let secret = CONFIG.secret_for_core().map(|s| s.to_owned());
        let level = Arc::clone(&self.ws_level);
        let reconnect = Arc::clone(&self.ws_reconnect);
        spawn_ws_logs(controller, secret, pending, level, reconnect);

        self.error = Some("Press p to start capturing logs".to_owned());
        // Fetch initial log level
        async {
            let cfg = tri!(
                tokio::task::spawn_blocking(config::fetch).await.unwrap(),
                or_set
            );
            wrapper(move |content: &mut Self| {
                if crate::config::is_core_mismatch() {
                    return;
                }
                let level = cfg
                    .log_level
                    .as_ref()
                    .map(|l| l.to_string())
                    .unwrap_or_else(|| "unknown".to_owned());
                content.current_log_level = level.clone();
                *content.ws_level.lock().unwrap() = level;
                content.error = None;
            })
        }
        .spawn_at(task_set);
    }

    fn handle_key_event(
        &mut self,
        key: Key,
        task_set: &mut FutureSet<Self>,
        _state: &mut Self::State,
    ) {
        match key {
            Key::MoveUp => {
                self.scroll = self.scroll.saturating_sub(1);
            }
            Key::MoveDown => {
                if self.scroll + 1 < self.buffer.count() {
                    self.scroll += 1;
                }
            }
            Key::GoTop => {
                self.scroll = 0;
            }
            Key::GoBottom => {
                if !self.buffer.is_empty() {
                    self.scroll = self.buffer.count().saturating_sub(1);
                }
            }
            Key::Search => {
                async move {
                    let filter = tri!(
                        Input::new()
                            .with_title("Filter".to_owned())
                            .build_and_send()
                            .await,
                        or_cancel
                    );
                    wrapper(move |content: &mut Logs| {
                        content.filter = (!filter.is_empty()).then_some(filter);
                    })
                }
                .spawn_at(task_set);
            }
            Key::TogglePause => {
                self.paused = !self.paused;
                if !self.paused {
                    // Kickstart the after_sync poll chain
                    async { wrapper(|_content: &mut Logs| {}) }.spawn_at(task_set);
                }
            }
            Key::FzfFind => {
                self.paused = true;
                let names: Vec<String> = self
                    .buffer
                    .iter_from_head()
                    .map(|e| format!("{} {} {}", e.time, e.type_, e.payload))
                    .collect();
                async move {
                    let selected = tokio::task::spawn_blocking(move || {
                        crate::tui::widget::fzffind::run_fzf(&names, "Find Log")
                    })
                    .await
                    .unwrap_or(None);
                    wrapper(move |content: &mut Logs| {
                        if let Some(idx) = selected {
                            content.scroll = idx;
                        }
                    })
                }
                .spawn_at(task_set);
            }
            Key::Clear => {
                self.buffer.clear();
                self.scroll = 0;
                self.filter = None;
            }
            Key::ToggleDebug => self.toggle_log_level("debug", task_set),
            Key::ToggleInfo => self.toggle_log_level("info", task_set),
            Key::ToggleWarning => self.toggle_log_level("warning", task_set),
            Key::ToggleError => self.toggle_log_level("error", task_set),
            Key::ToggleSilent => self.toggle_log_level("silent", task_set),
        }
    }

    fn render(&self, f: &mut Frame, area: Rect, _state: &mut Self::State) {
        let block = Block::bordered()
            .border_style(Theme::get().section("logs").border)
            .title(Self::TITLE);

        let mut title_parts = Vec::new();
        title_parts.push(self.current_log_level.clone());
        if let Some(ref filter) = self.filter {
            title_parts.push(format!(" / {filter} "));
        } else {
            title_parts.push(" /: Search ".to_owned());
        }
        if self.paused {
            title_parts.push(" [PAUSED]".to_owned());
        }
        let block = block.title_bottom(Line::raw(title_parts.join(" ")).right_aligned().reversed());

        if !self.error.as_deref().unwrap_or("").is_empty() && self.buffer.is_empty() {
            let widget =
                ratatui::widgets::Paragraph::new(self.error.as_deref().unwrap_or("")).block(block);
            f.render_widget(widget, area);
            return;
        }

        let visible_lines: Vec<ListItem> = self
            .buffer
            .iter_from_head()
            .map(|e| format!("{} {} {}", e.time, e.type_, e.payload))
            .filter(|line| self.filter.as_deref().is_none_or(|pat| line.contains(pat)))
            .map(|line| ListItem::new(Line::raw(line)))
            .collect();

        let highlight_style = Theme::get().section("logs").highlight;
        let list = List::new(visible_lines)
            .block(block)
            .highlight_style(highlight_style);

        let mut list_state =
            ratatui::widgets::ListState::default().with_selected(Some(self.scroll));
        f.render_stateful_widget(list, area, &mut list_state);
    }
}

impl Logs {
    fn toggle_log_level(&mut self, level: &str, _task_set: &mut FutureSet<Self>) {
        if crate::config::is_core_mismatch() {
            return;
        }
        let level = level.to_owned();
        self.current_log_level = level.clone();
        *self.ws_level.lock().unwrap() = level;
        self.ws_reconnect.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyCode;

    fn kev(code: KeyCode, shift: bool) -> crate::tui::Key {
        crate::tui::Key {
            code,
            shift,
            ctrl: false,
            alt: false,
            super_: false,
        }
    }

    fn make_entry(type_: &str, payload: &str, time: &str) -> LogEntry {
        LogEntry {
            type_: type_.to_owned(),
            payload: payload.to_owned(),
            time: time.to_owned(),
        }
    }

    #[test]
    fn key_j_maps_to_move_down() {
        let k = kev(KeyCode::Char('j'), false);
        let result = Key::try_from(&k);
        assert!(matches!(result, Ok(Key::MoveDown)));
    }

    #[test]
    fn key_k_maps_to_move_up() {
        let k = kev(KeyCode::Char('k'), false);
        let result = Key::try_from(&k);
        assert!(matches!(result, Ok(Key::MoveUp)));
    }

    #[test]
    fn key_p_maps_to_toggle_pause() {
        let k = kev(KeyCode::Char('p'), false);
        let result = Key::try_from(&k);
        assert!(matches!(result, Ok(Key::TogglePause)));
    }

    #[test]
    fn key_c_maps_to_clear() {
        let k = kev(KeyCode::Char('c'), false);
        let result = Key::try_from(&k);
        assert!(matches!(result, Ok(Key::Clear)));
    }

    #[test]
    fn key_f_maps_to_fzf_find() {
        let k = kev(KeyCode::Char('f'), false);
        let result = Key::try_from(&k);
        assert!(matches!(result, Ok(Key::FzfFind)));
    }

    #[test]
    fn key_slash_maps_to_search() {
        let k = kev(KeyCode::Char('/'), false);
        let result = Key::try_from(&k);
        assert!(matches!(result, Ok(Key::Search)));
    }

    #[test]
    fn default_logs_is_paused_true() {
        let logs = Logs::default();
        assert!(logs.paused, "log capture should be paused by default");
    }

    #[test]
    fn default_logs_has_empty_buffer() {
        let logs = Logs::default();
        assert!(logs.buffer.is_empty());
        assert_eq!(logs.scroll, 0);
    }

    #[test]
    fn clear_resets_state() {
        let mut logs = Logs {
            buffer: LogBuffer::new(),
            scroll: 1,
            filter: Some("test".to_owned()),
            ..Default::default()
        };
        logs.buffer
            .push(make_entry("info", "line1", "00-01-01 00:00:00"));
        logs.buffer
            .push(make_entry("info", "line2", "00-01-01 00:00:00"));
        assert_eq!(logs.buffer.count(), 2);
        assert!(logs.filter.is_some());

        logs.buffer.clear();
        logs.scroll = 0;
        logs.filter = None;

        assert!(logs.buffer.is_empty());
        assert_eq!(logs.scroll, 0);
        assert!(logs.filter.is_none());
    }

    #[test]
    fn toggle_pause_flips_state() {
        let mut logs = Logs::default();
        assert!(logs.paused, "default is paused");
        logs.paused = false;
        assert!(!logs.paused);
        logs.paused = true;
        assert!(logs.paused);
    }

    #[test]
    fn scroll_clamps_at_top() {
        let mut logs = Logs::default();
        logs.scroll = 0;
        logs.scroll = logs.scroll.saturating_sub(1);
        assert_eq!(logs.scroll, 0);
    }

    #[test]
    fn move_down_clamps_at_end() {
        let mut logs = Logs {
            buffer: LogBuffer::new(),
            scroll: 0,
            ..Default::default()
        };
        logs.buffer
            .push(make_entry("info", "a", "00-00-00 00:00:00"));
        logs.buffer
            .push(make_entry("info", "b", "00-00-00 00:00:00"));
        logs.scroll += 1;
        logs.scroll += 1;
        if logs.scroll >= logs.buffer.count() {
            logs.scroll = logs.buffer.count().saturating_sub(1);
        }
        assert_eq!(logs.scroll, 1);
    }

    #[test]
    fn log_buffer_push_and_iter() {
        let mut buf = LogBuffer::new();
        assert!(buf.is_empty());
        buf.push(make_entry("info", "first", "00-00-00 00:00:01"));
        buf.push(make_entry("warning", "second", "00-00-00 00:00:02"));
        assert_eq!(buf.count(), 2);

        let entries: Vec<&LogEntry> = buf.iter_from_head().collect();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].payload, "first");
        assert_eq!(entries[1].payload, "second");
    }

    #[test]
    fn log_buffer_wraps_at_capacity() {
        const SIZE: usize = LOG_BUFFER_SIZE;
        let mut buf = LogBuffer::new();
        for i in 0..(SIZE + 5) {
            buf.push(make_entry("info", &format!("line{i}"), "00-00-00 00:00:00"));
        }
        assert_eq!(buf.count(), SIZE);
        let entries: Vec<&LogEntry> = buf.iter_from_head().collect();
        assert_eq!(entries.len(), SIZE);
        assert_eq!(entries[0].payload, "line5");
        assert_eq!(entries[SIZE - 1].payload, format!("line{}", SIZE + 4));
    }

    #[test]
    fn log_buffer_clear() {
        let mut buf = LogBuffer::new();
        buf.push(make_entry("info", "test", "00-00-00 00:00:00"));
        assert!(!buf.is_empty());
        buf.clear();
        assert!(buf.is_empty());
        assert_eq!(buf.count(), 0);
    }

    #[test]
    fn parse_log_entries_valid_json() {
        let body = "{\"type\":\"info\",\"payload\":\"test message\"}";
        let entries = api_log::parse_log_entries(body);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].type_, "info");
        assert_eq!(entries[0].payload, "test message");
    }

    #[test]
    fn parse_log_entries_multiple_lines() {
        let body = "{\"type\":\"info\",\"payload\":\"first\"}\n{\"type\":\"warning\",\"payload\":\"second\"}";
        let entries = api_log::parse_log_entries(body);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].type_, "info");
        assert_eq!(entries[1].type_, "warning");
    }

    #[test]
    fn parse_log_entries_skips_invalid() {
        let body = "{\"type\":\"info\",\"payload\":\"valid\"}\nnot json\n{\"type\":\"error\",\"payload\":\"also valid\"}";
        let entries = api_log::parse_log_entries(body);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].payload, "valid");
        assert_eq!(entries[1].payload, "also valid");
    }

    #[test]
    fn parse_log_entries_empty_string() {
        let entries = api_log::parse_log_entries("");
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_log_entries_missing_fields() {
        let body = "{\"type\":\"info\"}";
        let entries = api_log::parse_log_entries(body);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].type_, "info");
        assert_eq!(entries[0].payload, "");
    }
}
