use super::dev::*;
use crate::config::CONFIG;
use crate::functions::restful::api_log::{self, LogEntry};
use crate::functions::restful::config;
use ratatui::text::Line;
use ratatui::widgets::{List, ListItem};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

newtype_tab!(LogsTab(Tab<Logs>), Logs);

key_map!(
    Logs,
    Action,
    DefaultBindings::new()
        .with_keys([
            (KeyCode::Up, Action::MoveUp),
            (KeyCode::Down, Action::MoveDown),
            (KeyCode::Char('k'), Action::MoveUp),
            (KeyCode::Char('j'), Action::MoveDown),
            (KeyCode::Char('G'), Action::GoBottom),
            (KeyCode::Char('/'), Action::Search),
            (KeyCode::Char('p'), Action::TogglePause),
            (KeyCode::Char('f'), Action::FzfFind),
            (KeyCode::Char('c'), Action::Clear),
        ])
        .with_chords(KeyCode::Char('g'), [(KeyCode::Char('g'), Action::GoTop)])
        .with_chords(
            KeyCode::Char('t'),
            [
                (KeyCode::Char('d'), Action::ToggleDebug),
                (KeyCode::Char('i'), Action::ToggleInfo),
                (KeyCode::Char('w'), Action::ToggleWarning),
                (KeyCode::Char('e'), Action::ToggleError),
            ]
        )
);

#[derive_aliases::derive(..Action, Debug)]
pub(in crate::tui) enum Action {
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
}

impl Document for Action {
    fn get_doc(&self) -> &'static str {
        use crate::tui::key::consts::*;
        match self {
            Self::MoveUp => MOVE_UP,
            Self::MoveDown => MOVE_DOWN,
            Self::GoTop => GO_TOP,
            Self::GoBottom => GO_BOTTOM,
            Self::Search => FILTER,
            Self::TogglePause => PAUSE,
            Self::FzfFind => "Find",
            Self::Clear => "Clear logs",
            Self::ToggleDebug => "Toggle Debug",
            Self::ToggleInfo => "Toggle Info",
            Self::ToggleWarning => "Toggle Warning",
            Self::ToggleError => "Toggle Error",
        }
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
            paused: Arc::new(AtomicBool::new(true)),
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
    paused: Arc<AtomicBool>,
    current_log_level: String,
    ws_pending: Option<Arc<Mutex<Vec<LogEntry>>>>,
    ws_level: Arc<Mutex<String>>,
    ws_reconnect: Arc<AtomicBool>,
}

fn queue_log_message(text: &str, pending: &Mutex<Vec<LogEntry>>, paused: &AtomicBool) {
    // Keep receiving while paused, but do not retain new messages for later replay.
    if paused.load(Ordering::Relaxed) {
        return;
    }
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
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

fn spawn_ws_logs(
    controller: String,
    secret: Option<String>,
    pending: Arc<Mutex<Vec<LogEntry>>>,
    level: Arc<Mutex<String>>,
    reconnect: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
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
                                queue_log_message(&text, &pending, &paused);
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
    type Action = Action;
    type State = ();

    const TITLE: &str = "Logs";

    fn on_enter(&mut self, task_set: &mut FutureSet<Self>, _state: &mut Self::State) {
        if crate::config::is_core_mismatch() {
            self.buffer.clear();
            self.error = Some("API data mismatch with configured core".to_owned());
            self.paused.store(true, Ordering::Relaxed);
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
        if self.paused.load(Ordering::Relaxed) {
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
        let paused = Arc::clone(&self.paused);
        spawn_ws_logs(controller, secret, pending, level, reconnect, paused);

        self.error = Some("Log capture is paused".to_owned());
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

    fn handle_action(
        &mut self,
        action: Action,
        task_set: &mut FutureSet<Self>,
        _state: &mut Self::State,
    ) {
        match action {
            Action::MoveUp => {
                self.scroll = self.scroll.saturating_sub(1);
            }
            Action::MoveDown => {
                if self.scroll + 1 < self.buffer.count() {
                    self.scroll += 1;
                }
            }
            Action::GoTop => {
                self.scroll = 0;
            }
            Action::GoBottom => {
                if !self.buffer.is_empty() {
                    self.scroll = self.buffer.count().saturating_sub(1);
                }
            }
            Action::Search => {
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
            Action::TogglePause => {
                let was_paused = self.paused.fetch_xor(true, Ordering::Relaxed);
                if was_paused {
                    // Kickstart the after_sync poll chain
                    async { wrapper(|_content: &mut Logs| {}) }.spawn_at(task_set);
                }
            }
            Action::FzfFind => {
                self.paused.store(true, Ordering::Relaxed);
                let names: Vec<String> = self
                    .buffer
                    .iter_from_head()
                    .map(|e| format!("{} {} {}", e.time, e.type_, e.payload))
                    .collect();
                async move {
                    let selected = FzfFinder::new(names)
                        .with_title("Find Log")
                        .build_and_send()
                        .await
                        .unwrap_or_default();
                    wrapper(move |content: &mut Logs| {
                        if let Some(idx) = selected {
                            content.scroll = idx;
                        }
                    })
                }
                .spawn_at(task_set);
            }
            Action::Clear => {
                self.buffer.clear();
                self.scroll = 0;
                self.filter = None;
            }
            Action::ToggleDebug => self.toggle_log_level("debug", task_set),
            Action::ToggleInfo => self.toggle_log_level("info", task_set),
            Action::ToggleWarning => self.toggle_log_level("warning", task_set),
            Action::ToggleError => self.toggle_log_level("error", task_set),
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
        }
        if self.paused.load(Ordering::Relaxed) {
            title_parts.push(" [PAUSED]".to_owned());
        }
        let block = if title_parts.len() > 1 {
            block.title_bottom(Line::raw(title_parts.join(" ")).right_aligned().reversed())
        } else {
            block
        };

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

    fn queue(payload: &str, pending: &Mutex<Vec<LogEntry>>, paused: &AtomicBool) {
        let text = serde_json::json!({"type": "info", "payload": payload}).to_string();
        queue_log_message(&text, pending, paused);
    }

    #[test]
    fn paused_messages_are_discarded_without_replay() {
        let logs = Logs::default();
        let pending = Mutex::new(Vec::new());

        queue("initially paused", &pending, &logs.paused);
        assert!(pending.lock().unwrap().is_empty());

        logs.paused.store(false, Ordering::Relaxed);
        queue("before pause", &pending, &logs.paused);
        logs.paused.store(true, Ordering::Relaxed);
        for _ in 0..1000 {
            queue("during pause", &pending, &logs.paused);
        }
        let entries = std::mem::take(&mut *pending.lock().unwrap());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].payload, "before pause");

        logs.paused.store(false, Ordering::Relaxed);
        queue("after resume", &pending, &logs.paused);
        let entries = pending.lock().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].payload, "after resume");
    }

    #[tokio::test]
    async fn pause_key_keeps_the_batch_already_scheduled_for_display() {
        let pending = Arc::new(Mutex::new(Vec::new()));
        let mut logs = Logs {
            ws_pending: Some(Arc::clone(&pending)),
            ..Logs::default()
        };
        let receiver_paused = Arc::clone(&logs.paused);
        let mut tasks = FutureSet::new();

        logs.handle_action(Action::TogglePause, &mut tasks, &mut ());
        let kickstart = tasks.join_next().await.unwrap().unwrap();
        kickstart(&mut logs);
        assert!(!receiver_paused.load(Ordering::Relaxed));
        queue("before pause", &pending, &receiver_paused);
        logs.after_sync(&mut tasks);
        let callback = tokio::time::timeout(Duration::from_secs(3), tasks.join_next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        logs.handle_action(Action::TogglePause, &mut tasks, &mut ());
        assert!(receiver_paused.load(Ordering::Relaxed));
        queue("during pause", &pending, &receiver_paused);
        callback(&mut logs);

        assert!(pending.lock().unwrap().is_empty());
        assert_eq!(logs.buffer.count(), 1);
        assert_eq!(
            logs.buffer.iter_from_head().next().unwrap().payload,
            "before pause"
        );
    }
}
