use crossterm::event::{self, Event};
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

const INPUT_CAPACITY: usize = 64;
const PAUSE_DRAIN_BATCH: usize = 64;
const CONTROL_LATENCY: Duration = Duration::from_millis(25);

#[derive(Default)]
struct State {
    paused: bool,
    stop: bool,
    pause_ack: Option<oneshot::Sender<()>>,
}

#[derive(Default)]
struct Control {
    state: Mutex<State>,
    changed: Condvar,
}

/// The only crossterm reader. Pause is acknowledged after any in-progress read
/// finishes, so a terminal worker never competes with a detached EventStream.
pub(in crate::tui) struct InputReader {
    control: Arc<Control>,
    receiver: mpsc::Receiver<io::Result<Event>>,
    worker: Option<tokio::task::JoinHandle<()>>,
}

impl InputReader {
    pub(in crate::tui) fn start() -> Self {
        Self::with_source(|timeout| {
            if event::poll(timeout)? {
                event::read().map(Some)
            } else {
                Ok(None)
            }
        })
    }

    fn with_source(
        mut source: impl FnMut(Duration) -> io::Result<Option<Event>> + Send + 'static,
    ) -> Self {
        let control = Arc::new(Control::default());
        let worker_control = control.clone();
        let (sender, receiver) = mpsc::channel(INPUT_CAPACITY);
        let worker = tokio::task::spawn_blocking(move || {
            let mut pending = None;
            let mut failed = false;
            loop {
                let mut state = worker_control.state.lock().unwrap();
                if state.stop {
                    break;
                }
                if state.paused {
                    pending = None;
                    // Crossterm may have parsed several events from one OS read.
                    // Discard that pre-handoff typeahead before acknowledging pause.
                    // Check shutdown between batches without locking during reads.
                    drop(state);
                    let mut drained = false;
                    let mut failure = None;
                    for _ in 0..PAUSE_DRAIN_BATCH {
                        match source(Duration::ZERO) {
                            Ok(Some(_)) => {}
                            Ok(None) => {
                                drained = true;
                                break;
                            }
                            Err(error) => {
                                failure = Some(error);
                                break;
                            }
                        }
                    }
                    state = worker_control.state.lock().unwrap();
                    if let Some(error) = failure {
                        log::warn!("Pause terminal input: {error}");
                        state.stop = true;
                        state.pause_ack = None;
                    }
                    if state.stop {
                        break;
                    }
                    if !drained {
                        continue;
                    }
                    if let Some(ack) = state.pause_ack.take() {
                        let _ = ack.send(());
                    }
                    while state.paused && !state.stop {
                        state = worker_control.changed.wait(state).unwrap();
                    }
                    continue;
                }
                if let Some(message) = pending.take() {
                    match sender.try_send(message) {
                        Ok(()) if failed => break,
                        Ok(()) => {}
                        Err(mpsc::error::TrySendError::Closed(_)) => break,
                        Err(mpsc::error::TrySendError::Full(message)) => {
                            pending = Some(message);
                            // next_event and control changes notify under the same mutex.
                            drop(worker_control.changed.wait(state).unwrap());
                            continue;
                        }
                    }
                }
                drop(state);
                match source(CONTROL_LATENCY) {
                    Ok(Some(event)) => pending = Some(Ok(event)),
                    Ok(None) => {}
                    Err(error) => {
                        failed = true;
                        pending = Some(Err(error));
                    }
                }
            }
        });
        Self {
            control,
            receiver,
            worker: Some(worker),
        }
    }

    pub(in crate::tui) async fn next_event(&mut self) -> Option<io::Result<Event>> {
        let event = self.receiver.recv().await;
        {
            let _state = self.control.state.lock().unwrap();
            self.control.changed.notify_all();
        }
        if event.is_none() && self.worker.is_some() {
            std::future::poll_fn(|cx| self.poll_finished(cx)).await;
        }
        event
    }

    pub(in crate::tui) fn request_pause(&mut self) -> oneshot::Receiver<()> {
        let (ack, receiver) = oneshot::channel();
        let mut state = self.control.state.lock().unwrap();
        let already_paused = state.paused && state.pause_ack.is_none();
        state.paused = true;
        if !state.stop {
            if already_paused {
                let _ = ack.send(());
            } else {
                state.pause_ack = Some(ack);
            }
        }
        self.control.changed.notify_all();
        // Pre-handoff keystrokes must not become shortcuts after terminal return.
        while self.receiver.try_recv().is_ok() {}
        receiver
    }

    pub(in crate::tui) fn resume(&mut self) {
        let mut state = self.control.state.lock().unwrap();
        if !state.stop {
            state.paused = false;
            state.pause_ack = None;
            self.control.changed.notify_all();
        }
    }

    pub(in crate::tui) fn begin_shutdown(&mut self) {
        let mut state = self.control.state.lock().unwrap();
        state.stop = true;
        state.pause_ack = None;
        self.receiver.close();
        while self.receiver.try_recv().is_ok() {}
        self.control.changed.notify_all();
    }

    #[allow(dead_code)] // shutdown waits on this after the event-loop refactor
    pub(in crate::tui) fn has_worker(&self) -> bool {
        self.worker.is_some()
    }

    pub(in crate::tui) fn poll_finished(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        let Some(worker) = self.worker.as_mut() else {
            return Poll::Pending;
        };
        match Pin::new(worker).poll(cx) {
            Poll::Ready(result) => {
                self.worker = None;
                if let Err(error) = result {
                    log::warn!("Terminal input worker: {error}");
                }
                Poll::Ready(())
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for InputReader {
    fn drop(&mut self) {
        self.begin_shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn pause_ack_stops_actual_reads_and_shutdown_reaps_worker() {
        let reads = Arc::new(AtomicUsize::new(0));
        let count = reads.clone();
        let mut input = InputReader::with_source(move |timeout| {
            count.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(timeout);
            Ok(None)
        });
        tokio::time::timeout(Duration::from_secs(1), input.request_pause())
            .await
            .unwrap()
            .unwrap();
        let paused_count = reads.load(Ordering::SeqCst);
        tokio::time::sleep(CONTROL_LATENCY * 2).await;
        assert_eq!(reads.load(Ordering::SeqCst), paused_count);
        input.resume();
        tokio::time::sleep(CONTROL_LATENCY * 2).await;
        assert!(reads.load(Ordering::SeqCst) > paused_count);
        input.begin_shutdown();
        tokio::time::timeout(
            Duration::from_secs(1),
            std::future::poll_fn(|cx| input.poll_finished(cx)),
        )
        .await
        .unwrap();
        assert!(!input.has_worker());
    }

    #[tokio::test]
    async fn pause_discards_all_buffered_input_and_accepts_new_input_after_resume() {
        let (sender, source) = std::sync::mpsc::channel();
        let old_event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        for _ in 0..500 {
            sender.send(old_event.clone()).unwrap();
        }
        let count = Arc::new(AtomicUsize::new(0));
        let produced = count.clone();
        let mut input =
            InputReader::with_source(move |timeout| match source.recv_timeout(timeout) {
                Ok(event) => {
                    produced.fetch_add(1, Ordering::SeqCst);
                    Ok(Some(event))
                }
                Err(_) => Ok(None),
            });
        tokio::time::timeout(Duration::from_secs(1), async {
            while count.load(Ordering::SeqCst) <= INPUT_CAPACITY {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        tokio::time::timeout(Duration::from_secs(1), input.request_pause())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 500);
        assert!(input.receiver.is_empty());

        let new_event = Event::Key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        sender.send(new_event.clone()).unwrap();
        input.resume();
        let received = tokio::time::timeout(Duration::from_secs(1), input.next_event())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(received, new_event);
        input.begin_shutdown();
        tokio::time::timeout(
            Duration::from_secs(1),
            std::future::poll_fn(|cx| input.poll_finished(cx)),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn continuous_input_delays_pause_but_shutdown_cancels_and_reaps_worker() {
        let draining = Arc::new(AtomicUsize::new(0));
        let count = draining.clone();
        let mut input = InputReader::with_source(move |timeout| {
            if timeout.is_zero() {
                count.fetch_add(1, Ordering::SeqCst);
            }
            Ok(Some(Event::Key(KeyEvent::new(
                KeyCode::Char('x'),
                KeyModifiers::NONE,
            ))))
        });
        let mut paused = input.request_pause();
        tokio::time::timeout(Duration::from_secs(1), async {
            while draining.load(Ordering::SeqCst) < PAUSE_DRAIN_BATCH * 3 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(matches!(
            paused.try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        ));
        input.begin_shutdown();
        assert!(paused.await.is_err());
        tokio::time::timeout(
            Duration::from_secs(1),
            std::future::poll_fn(|cx| input.poll_finished(cx)),
        )
        .await
        .unwrap();
        assert!(!input.has_worker());
    }
}
