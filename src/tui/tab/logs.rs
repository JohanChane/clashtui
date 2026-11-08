use super::dev::*;
use crate::config::CONFIG;
use crate::functions::restful::api_log::{self, LogEntry};
use crate::functions::restful::config;
use ratatui::text::Line;
use ratatui::widgets::{List, ListItem};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

newtype_tab!(LogsTab(Logs), Logs);

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
        let mut queue = pending.lock().unwrap();
        queue.push(LogEntry {
            type_,
            payload,
            time: api_log::timestamp(),
        });
        if queue.len() > LOG_BUFFER_SIZE {
            let overflow = queue.len() - LOG_BUFFER_SIZE;
            queue.drain(..overflow);
        }
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

impl Page for Logs {
    type Action = Action;
    type State = ();
    const TITLE: &str = "Logs";
    const TICK: Option<Duration> = Some(Duration::from_secs(1));

    fn on_event(&mut self, event: &Event, cx: &mut Cx<Self>) {
        match event {
            Event::Init => self.connect(),
            Event::Tick => self.drain(),
            Event::Shown | Event::Core(CoreEvent::StatusChanged(_) | CoreEvent::ConfigChanged) => {
                self.drain();
                self.check_core(cx);
            }
            _ => {}
        }
    }

    fn on_action(&mut self, action: Action, cx: &mut Cx<Self>, _state: &mut Self::State) {
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
                .spawn_at(cx);
            }
            Action::TogglePause => {
                self.paused.fetch_xor(true, Ordering::Relaxed);
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
                .spawn_at(cx);
            }
            Action::Clear => {
                self.buffer.clear();
                self.scroll = 0;
                self.filter = None;
            }
            Action::ToggleDebug => self.toggle_log_level("debug"),
            Action::ToggleInfo => self.toggle_log_level("info"),
            Action::ToggleWarning => self.toggle_log_level("warning"),
            Action::ToggleError => self.toggle_log_level("error"),
        }
    }

    fn render(&self, f: &mut Frame, area: Rect, _state: &mut Self::State, _focused: bool) {
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
    fn connect(&mut self) {
        if self.ws_pending.is_some() {
            return;
        }
        let pending = Arc::new(Mutex::new(Vec::new()));
        self.ws_pending = Some(Arc::clone(&pending));
        let controller = CONFIG.controller_for_core().to_owned();
        let secret = CONFIG.secret_for_core().map(|s| s.to_owned());
        let level = Arc::clone(&self.ws_level);
        let reconnect = Arc::clone(&self.ws_reconnect);
        let paused = Arc::clone(&self.paused);
        spawn_ws_logs(controller, secret, pending, level, reconnect, paused);
        self.error = Some("Log capture is paused".to_owned());
    }

    fn drain(&mut self) {
        let Some(pending) = &self.ws_pending else {
            return;
        };
        let entries: Vec<LogEntry> = pending.lock().unwrap().drain(..).collect();
        for entry in entries {
            self.buffer.push(entry);
        }
        if self.buffer.count() > 0 && self.scroll + 1 >= self.buffer.count().saturating_sub(1) {
            self.scroll = self.buffer.count().saturating_sub(1);
        }
    }

    /// When ready, fetch the log level; otherwise clear logs and pause.
    fn check_core(&mut self, cx: &mut Cx<Self>) {
        if let Err(reason) = monitor::status().ready() {
            self.buffer.clear();
            self.error = Some(reason);
            self.paused.store(true, Ordering::Relaxed);
            return;
        }
        async {
            let cfg = tri!(
                tokio::task::spawn_blocking(config::fetch).await.unwrap(),
                or_set
            );
            wrapper(move |content: &mut Self| {
                if monitor::status().ready().is_err() {
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
        .spawn_at(cx);
    }

    fn toggle_log_level(&mut self, level: &str) {
        if monitor::status().ready().is_err() {
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

    #[test]
    fn tick_drain_keeps_entries_queued_before_pause() {
        let pending = Arc::new(Mutex::new(Vec::new()));
        let mut logs = Logs {
            ws_pending: Some(Arc::clone(&pending)),
            ..Logs::default()
        };
        logs.paused.store(false, Ordering::Relaxed);
        queue("before pause", &pending, &logs.paused);
        logs.drain();
        logs.paused.store(true, Ordering::Relaxed);
        queue("during pause", &pending, &logs.paused);

        assert!(pending.lock().unwrap().is_empty());
        assert_eq!(logs.buffer.count(), 1);
        assert_eq!(
            logs.buffer.iter_from_head().next().unwrap().payload,
            "before pause"
        );
    }

    #[test]
    fn pending_queue_keeps_only_the_latest_entries() {
        let pending = Mutex::new(Vec::new());
        let paused = AtomicBool::new(false);
        for i in 0..(LOG_BUFFER_SIZE + 25) {
            queue(&format!("m{i}"), &pending, &paused);
        }
        let entries = pending.lock().unwrap();
        assert_eq!(entries.len(), LOG_BUFFER_SIZE);
        assert_eq!(entries[0].payload, "m25");
        assert_eq!(
            entries.last().unwrap().payload,
            format!("m{}", LOG_BUFFER_SIZE + 24)
        );
    }
}
