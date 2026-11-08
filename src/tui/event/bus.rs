use std::sync::Mutex;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use super::{Broadcast, Event};

/// Sender for the current Inbox. Without an Inbox (CLI or unit tests), emit is a no-op.
static SENDER: Mutex<Option<UnboundedSender<Event>>> = Mutex::new(None);

/// Receiver for the broadcast channel, owned by App::serve.
pub struct Inbox(UnboundedReceiver<Event>);

/// Enqueue a broadcast event. Safe to call from any thread.
pub fn emit(event: impl Broadcast) {
    let event = event.into();
    log::debug!("emit {event:?}");
    if let Some(sender) = SENDER.lock().unwrap().as_ref() {
        let _ = sender.send(event);
    }
}

impl Inbox {
    /// Install the global sender before starting any producers.
    pub fn open() -> Self {
        let (sender, receiver) = unbounded_channel();
        *SENDER.lock().unwrap() = Some(sender);
        Self(receiver)
    }

    pub async fn recv(&mut self) -> Option<Event> {
        self.0.recv().await
    }
}

impl Drop for Inbox {
    fn drop(&mut self) {
        *SENDER.lock().unwrap() = None;
    }
}
