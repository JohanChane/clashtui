use super::dev::*;
use crate::config::CONFIG;
use crate::functions::restful::api_log::{self, LogEntry};
use ratatui::text::Line;
use ratatui::widgets::{List, ListItem};
use std::cell::Cell;
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
    entries: [Option<(u64, LogEntry)>; LOG_BUFFER_SIZE],
    next_id: u64,
    tail: isize,
    len: usize,
}

impl LogBuffer {
    fn new() -> Self {
        const NONE: Option<(u64, LogEntry)> = None;
        Self {
            entries: [NONE; LOG_BUFFER_SIZE],
            next_id: 0,
            tail: -1,
            len: 0,
        }
    }

    fn push(&mut self, entry: LogEntry) {
        self.tail = (self.tail + 1) % LOG_BUFFER_SIZE as isize;
        self.entries[self.tail as usize] = Some((self.next_id, entry));
        self.next_id = self.next_id.wrapping_add(1);
        if self.len < LOG_BUFFER_SIZE {
            self.len += 1;
        }
    }

    fn clear(&mut self) {
        // Keep IDs unique across clears so an open finder cannot target a replacement log.
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
        self.iter_with_ids().map(|(_, entry)| entry)
    }

    fn iter_with_ids(&self) -> impl Iterator<Item = (u64, &LogEntry)> {
        let start = if self.len < LOG_BUFFER_SIZE {
            0
        } else {
            ((self.tail + 1) % LOG_BUFFER_SIZE as isize) as usize
        };
        let count = self.len;
        (0..count).filter_map(move |i| {
            let idx = (start + i) % LOG_BUFFER_SIZE;
            self.entries[idx].as_ref().map(|(id, entry)| (*id, entry))
        })
    }
}

impl Default for Logs {
    fn default() -> Self {
        Self {
            buffer: LogBuffer::new(),
            scroll: Cell::new(0),
            jump_target: Cell::new(None),
            error: None,
            filter: None,
            paused: Arc::new(AtomicBool::new(true)),
            ws_pending: None,
            subscription_level: Arc::new(Mutex::new("info".to_owned())),
        }
    }
}

struct Logs {
    buffer: LogBuffer,
    scroll: Cell<usize>,
    jump_target: Cell<Option<u64>>,
    error: Option<String>,
    filter: Option<String>,
    paused: Arc<AtomicBool>,
    ws_pending: Option<Arc<Mutex<Vec<LogEntry>>>>,
    subscription_level: Arc<Mutex<String>>,
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
        if paused.load(Ordering::Relaxed) {
            return;
        }
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

fn read_log_messages(
    mut read: impl FnMut() -> tungstenite::Result<tungstenite::Message>,
    level: &Mutex<String>,
    connected_level: &str,
    pending: &Mutex<Vec<LogEntry>>,
    paused: &AtomicBool,
) {
    loop {
        // Compare desired and connected levels on busy streams as well as read timeouts.
        if *level.lock().unwrap() != connected_level {
            return;
        }
        match read() {
            Ok(tungstenite::Message::Text(text)) => {
                if monitor::status().ready().is_ok() {
                    queue_log_message(&text, pending, paused);
                }
            }
            Ok(tungstenite::Message::Close(_)) => return,
            Err(tungstenite::Error::Io(ref e))
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(e) => {
                log::warn!("WebSocket read error: {e}");
                return;
            }
            _ => {}
        }
    }
}

fn spawn_ws_logs(
    controller: String,
    secret: Option<String>,
    pending: Arc<Mutex<Vec<LogEntry>>>,
    level: Arc<Mutex<String>>,
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

                    read_log_messages(|| ws.read(), &level, &current_level, &pending, &paused);
                }
                Err(e) => {
                    log::warn!("WebSocket connect error: {e}");
                }
            }
            if *level.lock().unwrap() != current_level {
                continue;
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

    fn on_event(&mut self, event: &Event, _cx: &mut Cx<Self>) {
        match event {
            Event::Init => self.connect(),
            Event::Tick => self.drain(),
            Event::Shown | Event::Core(CoreEvent::StatusChanged(_) | CoreEvent::ConfigChanged) => {
                if monitor::status().ready().is_ok() {
                    self.drain();
                }
                self.check_core();
            }
            _ => {}
        }
    }

    fn on_action(&mut self, action: Action, cx: &mut Cx<Self>, _state: &mut Self::State) {
        match action {
            Action::MoveUp => {
                self.scroll.set(self.scroll.get().saturating_sub(1));
            }
            Action::MoveDown => {
                if self.scroll.get() + 1 < self.buffer.count() {
                    self.scroll.set(self.scroll.get() + 1);
                }
            }
            Action::GoTop => {
                self.scroll.set(0);
            }
            Action::GoBottom => {
                if !self.buffer.is_empty() {
                    self.scroll.set(self.buffer.count().saturating_sub(1));
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
                let (names, ids) = self.finder_snapshot();
                async move {
                    let selected = FzfFinder::new(names)
                        .with_title("Find Log")
                        .build_and_send()
                        .await
                        .unwrap_or_default();
                    wrapper(move |content: &mut Logs| {
                        content
                            .set_finder_target(selected.and_then(|index| ids.get(index).copied()));
                    })
                }
                .spawn_at(cx);
            }
            Action::Clear => {
                self.buffer.clear();
                self.scroll.set(0);
                self.filter = None;
            }
            Action::ToggleDebug => self.toggle_log_level("debug"),
            Action::ToggleInfo => self.toggle_log_level("info"),
            Action::ToggleWarning => self.toggle_log_level("warning"),
            Action::ToggleError => self.toggle_log_level("error"),
        }
    }

    fn render(&self, f: &mut Frame, area: Rect, _state: &mut Self::State, _focused: bool) {
        // Tick and other callbacks may have changed the buffer after confirmation.
        if let Some(id) = self.jump_target.take()
            && let Some(index) = self
                .buffer
                .iter_with_ids()
                .position(|(candidate, _)| candidate == id)
        {
            self.scroll.set(index);
        }
        let block = Block::bordered()
            .border_style(Theme::get().section("logs").border)
            .title(Self::TITLE);

        let mut title_parts = Vec::new();
        title_parts.push(self.subscription_level.lock().unwrap().clone());
        if let Some(ref filter) = self.filter {
            title_parts.push(format!(" / {filter} "));
        }
        if self.paused.load(Ordering::Relaxed) {
            title_parts.push(" [PAUSED]".to_owned());
        }
        let block = if !title_parts.is_empty() {
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
            ratatui::widgets::ListState::default().with_selected(Some(self.scroll.get()));
        f.render_stateful_widget(list, area, &mut list_state);
    }
}

impl Logs {
    fn finder_snapshot(&mut self) -> (Vec<String>, Vec<u64>) {
        // Include pending logs without changing the user's capture state.
        self.drain();
        self.buffer
            .iter_with_ids()
            .map(|(id, entry)| {
                (
                    format!("{} {} {}", entry.time, entry.type_, entry.payload),
                    id,
                )
            })
            .unzip()
    }

    fn set_finder_target(&mut self, id: Option<u64>) {
        self.jump_target.set(id);
        if id.is_some() {
            self.filter = None;
        }
    }

    fn connect(&mut self) {
        if self.ws_pending.is_some() {
            return;
        }
        let pending = Arc::new(Mutex::new(Vec::new()));
        self.ws_pending = Some(Arc::clone(&pending));
        let controller = CONFIG.controller_for_core().to_owned();
        let secret = CONFIG.secret_for_core().map(|s| s.to_owned());
        let level = Arc::clone(&self.subscription_level);
        let paused = Arc::clone(&self.paused);
        spawn_ws_logs(controller, secret, pending, level, paused);
        self.error = Some("Log capture is paused".to_owned());
    }

    fn drain(&mut self) {
        let Some(pending) = &self.ws_pending else {
            return;
        };
        let entries: Vec<LogEntry> = pending.lock().unwrap().drain(..).collect();
        if entries.is_empty() {
            return;
        }
        for entry in entries {
            self.buffer.push(entry);
        }
        if self.buffer.count() > 0 && self.scroll.get() + 1 >= self.buffer.count().saturating_sub(1)
        {
            self.scroll.set(self.buffer.count().saturating_sub(1));
        }
    }

    /// Keep the subscription level independent of core availability and configuration.
    /// When unavailable, clear logs and pause; when ready, clear the error.
    fn check_core(&mut self) {
        if let Err(reason) = monitor::status().ready() {
            if let Some(pending) = &self.ws_pending {
                pending.lock().unwrap().clear();
            }
            self.buffer.clear();
            self.error = Some(reason);
            self.paused.store(true, Ordering::Relaxed);
        } else {
            self.error = None;
        }
    }

    fn toggle_log_level(&mut self, level: &str) {
        *self.subscription_level.lock().unwrap() = level.to_owned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_core_discards_stream_messages() {
        assert!(monitor::status().ready().is_err());
        let pending = Mutex::new(Vec::new());
        let mut messages = [
            tungstenite::Message::Text(r#"{"type":"info","payload":"stale"}"#.into()),
            tungstenite::Message::Close(None),
        ]
        .into_iter();
        read_log_messages(
            || Ok(messages.next().expect("reader must stop on close")),
            &Mutex::new("info".to_owned()),
            "info",
            &pending,
            &AtomicBool::new(false),
        );
        assert!(pending.lock().unwrap().is_empty());
    }

    #[test]
    fn unavailable_core_clears_pending_logs_without_changing_subscription() {
        assert!(monitor::status().ready().is_err());
        let pending = Arc::new(Mutex::new(Vec::new()));
        let mut logs = Logs {
            ws_pending: Some(Arc::clone(&pending)),
            ..Logs::default()
        };
        logs.paused.store(false, Ordering::Relaxed);
        logs.toggle_log_level("debug");
        queue("buffered", &pending, &logs.paused);
        logs.drain();
        queue("pending", &pending, &logs.paused);

        logs.check_core();

        assert!(logs.buffer.is_empty());
        assert!(pending.lock().unwrap().is_empty());
        assert!(logs.paused.load(Ordering::Relaxed));
        assert!(logs.error.is_some());
        assert_eq!(*logs.subscription_level.lock().unwrap(), "debug");
    }

    #[tokio::test]
    async fn subscription_level_survives_tab_and_core_config_events() {
        let mut host = crate::tui::page::Host::<Logs>::default();
        assert_eq!(*host.page().subscription_level.lock().unwrap(), "info");
        host.handle_action(
            Action::ToggleDebug,
            &mut crate::tui::commands::Commands::default(),
        );
        for event in [
            Event::Hidden,
            Event::Shown,
            Event::Core(CoreEvent::ConfigChanged),
        ] {
            host.deliver(&event);
            assert_eq!(*host.page().subscription_level.lock().unwrap(), "debug");
        }

        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(60, 10)).unwrap();
        terminal
            .draw(|frame| host.render(frame, frame.area(), true))
            .unwrap();
        let footer: String = (0..60)
            .map(|x| terminal.backend().buffer()[(x, 9)].symbol())
            .collect();
        assert!(footer.contains("debug"));
        assert!(!footer.contains("silent"));
    }

    #[test]
    fn stream_level_changes_are_detected_on_messages_and_timeouts() {
        use tungstenite::{Error, Message};
        for response in [
            Ok(Message::Text(r#"{"type":"info","payload":"busy"}"#.into())),
            Ok(Message::Ping(Vec::new().into())),
            Err(Error::Io(std::io::ErrorKind::TimedOut.into())),
            Err(Error::Io(std::io::ErrorKind::WouldBlock.into())),
        ] {
            let level = Mutex::new("info".to_owned());
            let pending = Mutex::new(Vec::new());
            let paused = AtomicBool::new(false);
            let mut response = Some(response);
            let mut reads = 0;
            read_log_messages(
                || {
                    reads += 1;
                    assert_eq!(
                        reads, 1,
                        "must reconnect before reading the old stream again"
                    );
                    *level.lock().unwrap() = "debug".into();
                    response.take().unwrap()
                },
                &level,
                "info",
                &pending,
                &paused,
            );
            assert_eq!(reads, 1);
            assert!(!paused.load(Ordering::Relaxed));
        }
    }

    #[test]
    fn level_changed_during_connect_is_detected_before_reading() {
        let level = Mutex::new("debug".to_owned());
        read_log_messages(
            || panic!("must reconnect using the new level before reading"),
            &level,
            "info",
            &Mutex::new(Vec::new()),
            &AtomicBool::new(true),
        );
    }

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
    fn empty_tick_preserves_the_finder_selection_while_paused() {
        let pending = Arc::new(Mutex::new(Vec::new()));
        let mut logs = Logs {
            ws_pending: Some(Arc::clone(&pending)),
            ..Logs::default()
        };
        logs.paused.store(false, Ordering::Relaxed);
        queue("first", &pending, &logs.paused);
        queue("second", &pending, &logs.paused);
        logs.paused.store(true, Ordering::Relaxed);
        logs.finder_snapshot();
        logs.scroll.set(0);
        logs.drain();
        assert_eq!(logs.scroll.get(), 0);
    }

    fn draw(logs: &Logs) {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(60, 10)).unwrap();
        terminal
            .draw(|frame| logs.render(frame, frame.area(), &mut (), true))
            .unwrap();
    }

    #[test]
    fn finder_preserves_capture_state_on_open_confirm_and_cancel() {
        for paused in [false, true] {
            let pending = Arc::new(Mutex::new(Vec::new()));
            let mut logs = Logs {
                ws_pending: Some(Arc::clone(&pending)),
                ..Logs::default()
            };
            logs.paused.store(false, Ordering::Relaxed);
            queue("before finder", &pending, &logs.paused);
            logs.paused.store(paused, Ordering::Relaxed);
            let (names, ids) = logs.finder_snapshot();
            assert_eq!(names.len(), 1);
            assert!(names[0].ends_with("before finder"));
            assert_eq!(logs.paused.load(Ordering::Relaxed), paused);

            queue("during finder", &pending, &logs.paused);
            logs.drain();
            assert_eq!(logs.buffer.count(), if paused { 1 } else { 2 });
            logs.set_finder_target(Some(ids[0]));
            draw(&logs);
            assert_eq!(logs.scroll.get(), 0);
            assert_eq!(logs.paused.load(Ordering::Relaxed), paused);

            logs.set_finder_target(None);
            draw(&logs);
            assert_eq!(logs.scroll.get(), 0);
            assert_eq!(logs.paused.load(Ordering::Relaxed), paused);
        }
    }

    #[test]
    fn finder_tracks_duplicate_logs_through_overflow_until_render() {
        let mut logs = Logs::default();
        for _ in 0..LOG_BUFFER_SIZE {
            logs.buffer.push(LogEntry {
                type_: "info".into(),
                payload: "same message".into(),
                time: "same time".into(),
            });
        }
        let (_, ids) = logs.finder_snapshot();
        logs.set_finder_target(Some(ids[20]));
        // Simulate a Tick after the finder callback, before the next render.
        for _ in 0..10 {
            logs.buffer.push(LogEntry {
                type_: "info".into(),
                payload: "same message".into(),
                time: "same time".into(),
            });
        }
        draw(&logs);
        assert_eq!(logs.scroll.get(), 10);
        assert_eq!(logs.buffer.iter_with_ids().nth(10).unwrap().0, ids[20]);

        // An evicted target must not redirect the cursor to a replacement row.
        logs.set_finder_target(Some(ids[0]));
        draw(&logs);
        assert_eq!(logs.scroll.get(), 10);
    }

    #[test]
    fn finder_target_does_not_survive_clear_and_refill() {
        let pending = Arc::new(Mutex::new(Vec::new()));
        let mut logs = Logs {
            ws_pending: Some(Arc::clone(&pending)),
            ..Logs::default()
        };
        logs.paused.store(false, Ordering::Relaxed);
        queue("old first", &pending, &logs.paused);
        queue("old second", &pending, &logs.paused);
        let (_, ids) = logs.finder_snapshot();
        logs.set_finder_target(Some(ids[1]));
        logs.buffer.clear();
        queue("new first", &pending, &logs.paused);
        queue("new second", &pending, &logs.paused);
        logs.drain();
        logs.scroll.set(0);
        draw(&logs);
        assert_eq!(logs.scroll.get(), 0);
        assert!(
            logs.buffer
                .iter_with_ids()
                .all(|(id, _)| !ids.contains(&id))
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
