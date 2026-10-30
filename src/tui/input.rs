//! The only crossterm input reader. Pause acknowledgements mean reads have stopped.

use crossterm::event::Event;
use std::{io, sync::mpsc, thread, time::Duration};
use tokio::sync::{mpsc as async_mpsc, oneshot};

const POLL_INTERVAL: Duration = Duration::from_millis(20);

enum Control {
    Pause(oneshot::Sender<()>),
    Resume,
    Stop,
}

trait EventSource: Send + 'static {
    fn poll(&mut self, timeout: Duration) -> io::Result<bool>;
    fn read(&mut self) -> io::Result<Event>;
    fn discard_pending(&mut self) {}
}

struct TerminalEvents;

impl EventSource for TerminalEvents {
    fn poll(&mut self, timeout: Duration) -> io::Result<bool> {
        crossterm::event::poll(timeout)
    }

    fn read(&mut self) -> io::Result<Event> {
        crossterm::event::read()
    }

    fn discard_pending(&mut self) {
        // crossterm can parse several keys in a single OS read. Discard those
        // before ownership changes, so repeated Enter cannot replay afterwards.
        for _ in 0..4096 {
            if !matches!(crossterm::event::poll(Duration::ZERO), Ok(true)) {
                break;
            }
            if crossterm::event::read().is_err() {
                break;
            }
        }
    }
}

pub(super) struct InputReader {
    control: mpsc::Sender<Control>,
    events: async_mpsc::UnboundedReceiver<io::Result<Event>>,
    worker: Option<thread::JoinHandle<()>>,
}

impl InputReader {
    pub fn start() -> Self {
        Self::with_source(TerminalEvents)
    }

    fn with_source(mut source: impl EventSource) -> Self {
        let (control, commands) = mpsc::channel();
        let (sender, events) = async_mpsc::unbounded_channel();
        let worker = thread::spawn(move || {
            let mut paused = false;
            loop {
                let command = if paused {
                    match commands.recv() {
                        Ok(command) => Some(command),
                        Err(_) => break,
                    }
                } else {
                    match commands.try_recv() {
                        Ok(command) => Some(command),
                        Err(mpsc::TryRecvError::Empty) => None,
                        Err(mpsc::TryRecvError::Disconnected) => break,
                    }
                };
                match command {
                    Some(Control::Pause(ack)) => {
                        paused = true;
                        source.discard_pending();
                        // Runs between completed poll/read operations. No input
                        // access occurs after the acknowledgement until resume.
                        let _ = ack.send(());
                        continue;
                    }
                    Some(Control::Resume) => paused = false,
                    Some(Control::Stop) => break,
                    None => (),
                }
                if paused {
                    continue;
                }
                let result = match source.poll(POLL_INTERVAL) {
                    Ok(true) => Some(source.read()),
                    Ok(false) => None,
                    Err(error) => Some(Err(error)),
                };
                if let Some(event) = result {
                    let failed = event.is_err();
                    if sender.send(event).is_err() || failed {
                        break;
                    }
                }
            }
        });
        Self {
            control,
            events,
            worker: Some(worker),
        }
    }

    pub async fn next(&mut self) -> Option<io::Result<Event>> {
        self.events.recv().await
    }

    pub async fn pause(&mut self) -> anyhow::Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.control
            .send(Control::Pause(sender))
            .map_err(|_| anyhow::anyhow!("Terminal input reader stopped"))?;
        receiver.await.map_err(|_| {
            anyhow::anyhow!("Terminal input reader stopped before pause acknowledgement")
        })?;
        // Nothing can arrive until resume, so this drains the old epoch.
        while self.events.try_recv().is_ok() {}
        Ok(())
    }

    pub fn resume(&mut self) -> anyhow::Result<()> {
        self.control
            .send(Control::Resume)
            .map_err(|_| anyhow::anyhow!("Terminal input reader stopped"))
    }
}

impl Drop for InputReader {
    fn drop(&mut self) {
        log::trace!("Stopping terminal input reader");
        let _ = self.control.send(Control::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        log::trace!("Terminal input reader stopped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    struct FakeEvents(Arc<AtomicUsize>);
    impl EventSource for FakeEvents {
        fn poll(&mut self, _timeout: Duration) -> io::Result<bool> {
            thread::sleep(Duration::from_millis(2));
            Ok(true)
        }
        fn read(&mut self) -> io::Result<Event> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(Event::Resize(80, 24))
        }
    }

    #[tokio::test]
    async fn acknowledged_pause_stops_reads_and_discards_queued_events() {
        let reads = Arc::new(AtomicUsize::new(0));
        let mut reader = InputReader::with_source(FakeEvents(reads.clone()));
        reader.next().await.unwrap().unwrap();
        reader.pause().await.unwrap();
        let paused_at = reads.load(Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert_eq!(reads.load(Ordering::SeqCst), paused_at);
        assert!(reader.events.try_recv().is_err());
        reader.resume().unwrap();
        reader.next().await.unwrap().unwrap();
        assert!(reads.load(Ordering::SeqCst) > paused_at);
    }

    #[tokio::test]
    async fn paused_reader_shuts_down_without_waiting_for_input() {
        let mut reader = InputReader::with_source(FakeEvents(Arc::new(AtomicUsize::new(0))));
        reader.pause().await.unwrap();
        let start = std::time::Instant::now();
        drop(reader);
        assert!(start.elapsed() < Duration::from_secs(1));
    }
}
