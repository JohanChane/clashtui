use crate::functions::restful::api_log::{self, LogEntry};
use std::collections::VecDeque;
use std::io;
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tungstenite::client::IntoClientRequest;

const LOG_BUFFER_SIZE: usize = 300;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
const IO_TIMEOUT: Duration = Duration::from_millis(500);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

pub(in crate::tui) struct Snapshot {
    pub entries: Vec<Arc<LogEntry>>,
    pub paused: bool,
    pub level: String,
    pub diagnostic: Option<String>,
}

struct CaptureState {
    entries: VecDeque<Arc<LogEntry>>,
    paused: bool,
    allowed: bool,
    stopped: bool,
    level: String,
    level_selected: bool,
    diagnostic: Option<String>,
    session: Arc<()>,
    socket: Option<TcpStream>,
}

impl Default for CaptureState {
    fn default() -> Self {
        Self {
            entries: VecDeque::with_capacity(LOG_BUFFER_SIZE),
            paused: true,
            allowed: false,
            stopped: false,
            level: "info".to_owned(),
            level_selected: false,
            diagnostic: Some("Waiting for API confirmation".to_owned()),
            session: Arc::new(()),
            socket: None,
        }
    }
}

impl CaptureState {
    fn accepts(&self, session: &Arc<()>) -> bool {
        !self.stopped && self.allowed && Arc::ptr_eq(&self.session, session)
    }

    fn invalidate_session(&mut self) {
        self.session = Arc::new(());
        if let Some(socket) = self.socket.take() {
            let _ = socket.shutdown(Shutdown::Both);
        }
    }
}

struct Shared {
    state: Mutex<CaptureState>,
    control: Condvar,
    changed: Arc<Notify>,
}

impl Shared {
    fn new() -> Self {
        Self {
            state: Mutex::new(CaptureState::default()),
            control: Condvar::new(),
            changed: Arc::new(Notify::new()),
        }
    }

    fn notify(&self) {
        self.control.notify_all();
        self.changed.notify_one();
    }

    fn current(&self, session: &Arc<()>) -> bool {
        self.state.lock().unwrap().accepts(session)
    }

    fn set_diagnostic(&self, session: &Arc<()>, diagnostic: Option<String>) {
        let mut state = self.state.lock().unwrap();
        if state.accepts(session) && state.diagnostic != diagnostic {
            state.diagnostic = diagnostic;
            drop(state);
            self.changed.notify_one();
        }
    }

    fn record(&self, session: &Arc<()>, text: &str) {
        {
            let state = self.state.lock().unwrap();
            if !state.accepts(session) || state.paused {
                return;
            }
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
            return;
        };
        let entry = LogEntry {
            type_: value["type"].as_str().unwrap_or("unknown").to_owned(),
            payload: value["payload"].as_str().unwrap_or("").to_owned(),
            time: api_log::timestamp(),
        };
        let mut state = self.state.lock().unwrap();
        // Keep receiving while paused, but do not retain new messages for later replay.
        // A new connection identity on every pause boundary rejects reads already in progress.
        if !state.accepts(session) || state.paused {
            return;
        }
        if state.entries.len() == LOG_BUFFER_SIZE {
            state.entries.pop_front();
        }
        state.entries.push_back(Arc::new(entry));
        drop(state);
        self.changed.notify_one();
    }

    fn install_socket(&self, session: &Arc<()>, stream: &TcpStream) -> io::Result<()> {
        let mut state = self.state.lock().unwrap();
        if !state.accepts(session) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "log connection invalidated",
            ));
        }
        state.socket = Some(stream.try_clone()?);
        Ok(())
    }

    fn clear_socket(&self, session: &Arc<()>) {
        let mut state = self.state.lock().unwrap();
        if Arc::ptr_eq(&state.session, session) {
            state.socket = None;
        }
    }
}

#[derive(Clone)]
struct Endpoint {
    controller: String,
    secret: Option<String>,
}

/// Application-owned capture. Hiding the view never stops this owner or clears its buffer.
pub(in crate::tui) struct LogCollector {
    shared: Arc<Shared>,
    endpoint: Option<Endpoint>,
    worker: Option<JoinHandle<()>>,
}

impl LogCollector {
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Shared::new()),
            endpoint: None,
            worker: None,
        }
    }

    pub fn notification(&self) -> Arc<Notify> {
        Arc::clone(&self.shared.changed)
    }

    pub fn set_core_state(
        &mut self,
        allowed: bool,
        diagnostic: Option<String>,
        level: Option<&str>,
    ) -> bool {
        let mut state = self.shared.state.lock().unwrap();
        if state.stopped {
            return false;
        }
        let mut changed = state.allowed != allowed;
        if let Some(level) = level.filter(|_| !state.level_selected)
            && state.level != level
        {
            state.level = level.to_owned();
            changed = true;
        }
        if !allowed && state.diagnostic != diagnostic {
            state.diagnostic = diagnostic;
            changed = true;
        } else if allowed && !state.allowed {
            state.diagnostic = Some("Connecting to log stream".to_owned());
        }
        state.allowed = allowed;
        if changed {
            state.invalidate_session();
        }
        drop(state);
        if allowed && self.worker.is_none() {
            self.start();
        }
        if changed {
            self.shared.notify();
        }
        changed
    }

    fn start(&mut self) {
        let endpoint = self.endpoint.clone().unwrap_or_else(|| Endpoint {
            controller: crate::config::CONFIG.controller_for_core().to_owned(),
            secret: crate::config::CONFIG.secret_for_core().map(str::to_owned),
        });
        let shared = Arc::clone(&self.shared);
        self.worker = Some(tokio::task::spawn_blocking(move || {
            capture(shared, endpoint)
        }));
    }

    /// Synchronous invalidation happens before publishing core changes, including when hidden.
    pub fn invalidate_connection(&mut self) -> bool {
        let mut state = self.shared.state.lock().unwrap();
        if state.stopped {
            return false;
        }
        let diagnostic = Some("API state is changing".to_owned());
        let changed = state.allowed || state.diagnostic != diagnostic;
        state.allowed = false;
        state.diagnostic = diagnostic;
        state.invalidate_session();
        drop(state);
        self.shared.notify();
        changed
    }

    pub(in crate::tui) fn snapshot(&self) -> Snapshot {
        let state = self.shared.state.lock().unwrap();
        Snapshot {
            entries: state.entries.iter().cloned().collect(),
            paused: state.paused,
            level: state.level.clone(),
            diagnostic: state.diagnostic.clone(),
        }
    }

    pub(in crate::tui) fn set_paused(&mut self, paused: bool) {
        let mut state = self.shared.state.lock().unwrap();
        if state.paused == paused || state.stopped {
            return;
        }
        state.paused = paused;
        // Closing the old stream also drops buffered frames from before the pause boundary.
        state.invalidate_session();
        drop(state);
        self.shared.notify();
    }

    pub(in crate::tui) fn set_level(&mut self, level: &str) {
        let mut state = self.shared.state.lock().unwrap();
        if state.stopped || !state.allowed {
            return;
        }
        state.level_selected = true;
        if state.level != level {
            state.level = level.to_owned();
            state.invalidate_session();
            drop(state);
            self.shared.notify();
        }
    }

    pub(in crate::tui) fn clear(&mut self) {
        self.shared.state.lock().unwrap().entries.clear();
        self.shared.changed.notify_one();
    }

    pub fn begin_shutdown(&mut self) {
        let mut state = self.shared.state.lock().unwrap();
        state.stopped = true;
        state.allowed = false;
        state.invalidate_session();
        drop(state);
        self.shared.notify();
    }

    pub fn has_worker(&self) -> bool {
        self.worker.is_some()
    }

    /// Polling the actual JoinHandle supplies a completion wake after the worker really exits.
    pub fn poll_finished(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        let Some(worker) = self.worker.as_mut() else {
            return Poll::Pending;
        };
        match std::pin::Pin::new(worker).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => {
                self.worker = None;
                if let Err(error) = result {
                    let mut state = self.shared.state.lock().unwrap();
                    state.allowed = false;
                    state.diagnostic = Some(format!("Log collector failed: {error}"));
                    drop(state);
                    self.shared.changed.notify_one();
                }
                Poll::Ready(())
            }
        }
    }
}

impl Drop for LogCollector {
    fn drop(&mut self) {
        self.begin_shutdown();
        // App drains poll_finished before dropping this owner; shutdown is also a fallback.
        if self
            .worker
            .as_ref()
            .is_some_and(|worker| !worker.is_finished())
        {
            log::warn!("Log collector dropped before its worker was reaped");
        }
    }
}

fn capture(shared: Arc<Shared>, endpoint: Endpoint) {
    let mut backoff = Duration::from_secs(1);
    loop {
        let (session, level) = {
            let mut state = shared.state.lock().unwrap();
            while !state.stopped && !state.allowed {
                state = shared.control.wait(state).unwrap();
            }
            if state.stopped {
                return;
            }
            (Arc::clone(&state.session), state.level.clone())
        };
        shared.set_diagnostic(&session, Some("Connecting to log stream".to_owned()));
        let started = Instant::now();
        let outcome = read_connection(&shared, &session, &endpoint, &level);
        shared.clear_socket(&session);
        if let Err(error) = outcome {
            shared.set_diagnostic(&session, Some(format!("Log connection: {error}")));
        }
        let state = shared.state.lock().unwrap();
        if state.stopped {
            return;
        }
        if !state.accepts(&session) {
            backoff = Duration::from_secs(1);
            continue;
        }
        if started.elapsed() >= Duration::from_secs(10) {
            backoff = Duration::from_secs(1);
        }
        let _ = shared
            .control
            .wait_timeout_while(state, backoff, |state| state.accepts(&session))
            .unwrap();
        backoff = (backoff * 2).min(Duration::from_secs(8));
    }
}

fn read_connection(
    shared: &Shared,
    session: &Arc<()>,
    endpoint: &Endpoint,
    level: &str,
) -> anyhow::Result<()> {
    let scheme = if endpoint.controller.starts_with("https://") {
        "wss"
    } else {
        "ws"
    };
    // Strip http(s):// prefix and trailing slash if any.
    let address = endpoint
        .controller
        .strip_prefix("http://")
        .or_else(|| endpoint.controller.strip_prefix("https://"))
        .unwrap_or(&endpoint.controller)
        .trim_end_matches('/');
    let mut request = format!("{scheme}://{address}/logs?level={level}").into_client_request()?;
    // This build does not enable a tungstenite TLS backend, matching the existing dependency.
    anyhow::ensure!(scheme == "ws", "TLS is not enabled for the log WebSocket");
    if let Some(secret) = &endpoint.secret {
        request.headers_mut().insert(
            tungstenite::http::header::AUTHORIZATION,
            format!("Bearer {secret}").parse()?,
        );
    }
    let host = request
        .uri()
        .host()
        .ok_or_else(|| anyhow::anyhow!("missing log host"))?
        .trim_matches(['[', ']']);
    let port = request.uri().port_u16().unwrap_or(80);
    let addresses = (host, port).to_socket_addrs()?;
    let mut stream = None;
    let mut last_error =
        io::Error::new(io::ErrorKind::AddrNotAvailable, "log host has no addresses");
    for address in addresses {
        if !shared.current(session) {
            return Ok(());
        }
        match TcpStream::connect_timeout(&address, CONNECT_TIMEOUT) {
            Ok(connected) => {
                stream = Some(connected);
                break;
            }
            Err(error) => last_error = error,
        }
    }
    let stream = stream.ok_or(last_error)?;
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    stream.set_nodelay(true)?;
    shared.install_socket(session, &stream)?;
    let config = tungstenite::protocol::WebSocketConfig::default()
        .max_message_size(Some(256 * 1024))
        .max_frame_size(Some(256 * 1024));
    let deadline = Instant::now() + HANDSHAKE_TIMEOUT;
    let mut handshake = tungstenite::client::client_with_config(request, stream, Some(config));
    let mut socket = loop {
        if !shared.current(session) {
            return Ok(());
        }
        match handshake {
            Ok((socket, _)) => break socket,
            Err(tungstenite::HandshakeError::Interrupted(mid)) if Instant::now() < deadline => {
                handshake = mid.handshake();
            }
            Err(tungstenite::HandshakeError::Interrupted(_)) => {
                anyhow::bail!("WebSocket handshake timed out")
            }
            Err(tungstenite::HandshakeError::Failure(error)) => return Err(error.into()),
        }
    };
    shared.set_diagnostic(session, None);
    while shared.current(session) {
        match socket.read() {
            Ok(tungstenite::Message::Text(text)) => shared.record(session, &text),
            Ok(tungstenite::Message::Close(_)) => anyhow::bail!("stream closed; reconnecting"),
            Ok(_) => {}
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                // An idle stream is healthy; only explicit control or a transport error reconnects.
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    fn collector() -> LogCollector {
        LogCollector::new()
    }

    fn active_session(collector: &LogCollector) -> Arc<()> {
        let mut state = collector.shared.state.lock().unwrap();
        state.allowed = true;
        state.paused = false;
        Arc::clone(&state.session)
    }

    fn record(collector: &LogCollector, session: &Arc<()>, message: &str) {
        collector.shared.record(
            session,
            &serde_json::json!({
                "type": "info", "payload": message,
            })
            .to_string(),
        );
    }

    #[test]
    fn capture_is_bounded_without_a_view() {
        let collector = collector();
        let session = active_session(&collector);
        for number in 0..LOG_BUFFER_SIZE + 20 {
            record(&collector, &session, &number.to_string());
        }
        let snapshot = collector.snapshot();
        assert_eq!(snapshot.entries.len(), LOG_BUFFER_SIZE);
        assert_eq!(snapshot.entries.first().unwrap().payload, "20");
        assert_eq!(snapshot.entries.last().unwrap().payload, "319");
    }

    #[test]
    fn snapshots_share_records_and_release_them_after_the_last_view_drops() {
        let mut collector = collector();
        let session = active_session(&collector);
        record(&collector, &session, "shared history");
        let first = collector.snapshot();
        let second = collector.snapshot();
        assert!(Arc::ptr_eq(&first.entries[0], &second.entries[0]));
        assert!(Arc::ptr_eq(
            &first.entries[0],
            &collector.shared.state.lock().unwrap().entries[0],
        ));

        let retained = Arc::downgrade(&first.entries[0]);
        collector.clear();
        assert!(collector.snapshot().entries.is_empty());
        assert_eq!(first.entries[0].payload, "shared history");
        drop(first);
        assert!(retained.upgrade().is_some());
        drop(second);
        assert!(retained.upgrade().is_none());
    }

    #[test]
    fn pause_boundaries_reject_reads_already_in_progress() {
        let mut collector = collector();
        let before = active_session(&collector);
        record(&collector, &before, "before pause");
        collector.set_paused(true);
        let during = Arc::clone(&collector.shared.state.lock().unwrap().session);
        record(&collector, &during, "while paused");
        collector.set_paused(false);
        record(&collector, &before, "late before pause");
        record(&collector, &during, "late during pause");
        let after = Arc::clone(&collector.shared.state.lock().unwrap().session);
        record(&collector, &after, "after resume");
        let messages: Vec<_> = collector
            .snapshot()
            .entries
            .into_iter()
            .map(|entry| entry.payload.clone())
            .collect();
        assert_eq!(messages, ["before pause", "after resume"]);
    }

    #[test]
    fn invalidation_rejects_old_records_and_diagnostics_without_clearing_pause() {
        let mut collector = collector();
        active_session(&collector);
        collector.set_paused(true);
        let old_session = Arc::clone(&collector.shared.state.lock().unwrap().session);
        collector.invalidate_connection();
        assert!(collector.snapshot().paused);
        {
            let mut state = collector.shared.state.lock().unwrap();
            state.allowed = true;
            state.diagnostic = Some("current diagnostic".to_owned());
        }
        record(&collector, &old_session, "old connection");
        collector
            .shared
            .set_diagnostic(&old_session, Some("old error".to_owned()));
        let snapshot = collector.snapshot();
        assert!(snapshot.entries.is_empty());
        assert!(snapshot.paused);
        assert_eq!(snapshot.diagnostic.as_deref(), Some("current diagnostic"));
    }

    #[test]
    fn invalidated_connection_cannot_append_after_api_recovers() {
        let mut collector = collector();
        let old_session = active_session(&collector);
        record(&collector, &old_session, "before stop");
        collector.invalidate_connection();
        let new_session = active_session(&collector);
        record(&collector, &old_session, "late stopped connection");
        record(&collector, &new_session, "after restart");
        let messages: Vec<_> = collector
            .snapshot()
            .entries
            .into_iter()
            .map(|entry| entry.payload.clone())
            .collect();
        assert_eq!(messages, ["before stop", "after restart"]);
    }

    #[tokio::test]
    async fn a_diagnostic_change_wakes_without_a_log_record() {
        let collector = collector();
        let session = active_session(&collector);
        let notification = collector.notification();
        collector
            .shared
            .set_diagnostic(&session, Some("connection failed".to_owned()));
        tokio::time::timeout(Duration::from_secs(1), notification.notified())
            .await
            .unwrap();
        assert!(collector.snapshot().entries.is_empty());
        assert_eq!(
            collector.snapshot().diagnostic.as_deref(),
            Some("connection failed")
        );
        collector
            .shared
            .set_diagnostic(&session, Some("retry failed".to_owned()));
        tokio::time::timeout(Duration::from_secs(1), notification.notified())
            .await
            .unwrap();
    }

    async fn wait_until(collector: &LogCollector, ready: impl Fn(&Snapshot) -> bool) {
        let notification = collector.notification();
        tokio::time::timeout(Duration::from_secs(3), async {
            while !ready(&collector.snapshot()) {
                notification.notified().await;
            }
        })
        .await
        .unwrap();
    }

    async fn stop(collector: &mut LogCollector) {
        collector.begin_shutdown();
        if collector.has_worker() {
            tokio::time::timeout(
                Duration::from_secs(2),
                std::future::poll_fn(|cx| collector.poll_finished(cx)),
            )
            .await
            .unwrap();
        }
        assert!(!collector.has_worker());
    }

    #[tokio::test]
    async fn idle_websocket_is_healthy_and_shutdown_wakes_its_read() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::task::spawn_blocking(move || {
            let (stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut socket = tungstenite::accept(stream).unwrap();
            socket
                .send(tungstenite::Message::Text(
                    r#"{"type":"info","payload":"captured without view"}"#.into(),
                ))
                .unwrap();
            let _ = socket.read();
        });
        let mut collector = collector();
        collector.endpoint = Some(Endpoint {
            controller: format!("http://{address}"),
            secret: None,
        });
        collector.set_paused(false);
        collector.set_core_state(true, None, Some("info"));
        wait_until(&collector, |snapshot| !snapshot.entries.is_empty()).await;
        tokio::time::sleep(IO_TIMEOUT * 2).await;
        assert!(
            collector.snapshot().diagnostic.is_none(),
            "idle timeout must not report core failure"
        );
        assert_eq!(
            collector.snapshot().entries[0].payload,
            "captured without view"
        );
        stop(&mut collector).await;
        server.await.unwrap();
    }

    #[tokio::test]
    async fn failed_connection_reports_diagnostic_and_shutdown_interrupts_backoff() {
        let unused = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = unused.local_addr().unwrap();
        drop(unused);
        let mut collector = collector();
        collector.endpoint = Some(Endpoint {
            controller: format!("http://{address}"),
            secret: None,
        });
        collector.set_core_state(true, None, None);
        wait_until(&collector, |snapshot| {
            snapshot
                .diagnostic
                .as_deref()
                .is_some_and(|value| value.starts_with("Log connection:"))
        })
        .await;
        assert!(collector.snapshot().entries.is_empty());
        stop(&mut collector).await;
    }
}
