use crate::tui::Key;
use ratatui::prelude::{Frame, Rect};
use ratatui::widgets::Block;
use std::cell::RefCell;
use std::sync::Mutex;
use tokio::sync::{mpsc, oneshot::Sender};

mod builder;
mod wrapper;

pub use builder::MsgBuilder;
use wrapper::Prompt;
pub(in crate::tui) use wrapper::Wrapped;

const CAPACITY: usize = 64;
static SENDER: Mutex<Option<mpsc::Sender<Wrapped>>> = Mutex::new(None);

thread_local! {
    // App callbacks hand these messages straight to their own PopUp after returning.
    // They never send into the bounded queue consumed by that same App task.
    static LOCAL_MESSAGES: RefCell<Option<Vec<Wrapped>>> = const { RefCell::new(None) };
}

pub(in crate::tui) fn with_local_messages<R>(f: impl FnOnce() -> R) -> (R, Vec<Wrapped>) {
    struct Scope(Option<Vec<Wrapped>>);
    impl Drop for Scope {
        fn drop(&mut self) {
            LOCAL_MESSAGES.with(|messages| *messages.borrow_mut() = self.0.take());
        }
    }
    let previous = LOCAL_MESSAGES.with(|messages| messages.replace(Some(Vec::new())));
    let _scope = Scope(previous);
    let result = f();
    let messages = LOCAL_MESSAGES.with(|messages| messages.borrow_mut().take().unwrap());
    (result, messages)
}

fn send_local(message: Wrapped) -> Result<(), Wrapped> {
    LOCAL_MESSAGES.with(|messages| {
        let mut messages = messages.borrow_mut();
        if let Some(local) = messages.as_mut() {
            local.push(message);
            Ok(())
        } else {
            Err(message)
        }
    })
}

fn sender() -> Option<mpsc::Sender<Wrapped>> {
    SENDER.lock().unwrap().clone()
}

fn send_blocking(message: Wrapped) -> Result<(), Wrapped> {
    let message = match send_local(message) {
        Ok(()) => return Ok(()),
        Err(message) => message,
    };
    match sender() {
        Some(sender) => sender.blocking_send(message).map_err(|error| error.0),
        None => Err(message),
    }
}

async fn send_async(message: Wrapped) -> Result<(), Wrapped> {
    let message = match send_local(message) {
        Ok(()) => return Ok(()),
        Err(message) => message,
    };
    match sender() {
        Some(sender) => sender.send(message).await.map_err(|error| error.0),
        None => Err(message),
    }
}

pub enum Route {
    Keep,
    Send,
    Drop,
}

pub trait Msg {
    type Result;

    /// Notifications may be logged when shutdown prevents displaying them.
    /// Interactive dialogs keep their potentially sensitive contents private.
    fn is_notification(&self) -> bool {
        false
    }

    fn match_key_event(&mut self, kv: &Key) -> Route;
    fn send(self, tx: Sender<Self::Result>);
    fn render(&self, f: &mut Frame, area: Rect, block: Block, is_focused: bool);
    /// (Width, Height)
    fn size(&self) -> (u16, u16);
}

pub struct PopUp {
    content: Vec<Wrapped>,
    receiver: mpsc::Receiver<Wrapped>,
    sender: mpsc::Sender<Wrapped>,
}

impl Default for PopUp {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel(CAPACITY);
        *SENDER.lock().unwrap() = Some(sender.clone());
        Self {
            content: Vec::new(),
            receiver,
            sender,
        }
    }
}

impl PopUp {
    pub fn check(&self) -> bool {
        !self.content.is_empty()
    }

    pub(in crate::tui) async fn next_message(&mut self) -> Option<Wrapped> {
        // Keep backpressure all the way to background producers. Otherwise App
        // would drain the bounded channel into an unbounded active stack.
        // Synchronous App callbacks still enter directly: they are finite input
        // or completion batches and cannot wait for their own receiver.
        if self.content.len() >= CAPACITY && !self.receiver.is_closed() {
            std::future::pending::<()>().await;
        }
        self.receiver.recv().await
    }

    pub(in crate::tui) fn accept(&mut self, message: Wrapped) {
        if !self.receiver.is_closed() {
            self.content.push(message);
        } else {
            message.log_undelivered();
        }
    }

    pub(in crate::tui) fn begin_shutdown(&mut self) {
        self.receiver.close();
        for message in self.content.drain(..) {
            message.log_undelivered();
        }
        while let Ok(message) = self.receiver.try_recv() {
            message.log_undelivered();
        }
    }

    pub fn handle_key_event(&mut self, kv: &Key) {
        if let Some(instance) = self.content.last_mut() {
            match instance.handle_key_event(kv) {
                Route::Keep => {}
                Route::Send => {
                    self.content.pop().unwrap().send();
                }
                Route::Drop => {
                    let _ = self.content.pop();
                }
            }
        }
    }

    /// `area` is not needed but kept for the widget interface.
    pub fn render(&mut self, f: &mut Frame, _: Rect) {
        self.content.iter_mut().for_each(|c| c.render(f));
    }
}

impl Drop for PopUp {
    fn drop(&mut self) {
        self.begin_shutdown();
        let mut sender = SENDER.lock().unwrap();
        if sender
            .as_ref()
            .is_some_and(|sender| sender.same_channel(&self.sender))
        {
            *sender = None;
        }
    }
}

struct Instance<C: Msg> {
    content: C,
    title: String,
    prompt: Option<Prompt>,
    is_focus_prompt: bool,
    tx: Sender<C::Result>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::popmsg::prelude::Confirm;
    use std::time::Duration;

    #[tokio::test]
    async fn local_callbacks_bypass_backpressure_and_shutdown_cancels_replies() {
        let mut popup = PopUp::default();
        for _ in 0..CAPACITY {
            let _reply = Confirm::title("active".into()).build_and_send_async().await;
            let message = popup.next_message().await.unwrap();
            popup.accept(message);
        }
        for _ in 0..CAPACITY {
            let _reply = Confirm::title("queued".into()).build_and_send_async().await;
        }
        let waiting = tokio::spawn(async {
            (Confirm::title("waiting".into())
                .build_and_send_async()
                .await,)
        });
        tokio::task::yield_now().await;
        assert!(!waiting.is_finished());
        assert!(
            tokio::time::timeout(Duration::from_millis(10), popup.next_message())
                .await
                .is_err()
        );
        assert_eq!(popup.content.len(), CAPACITY);
        assert!(!waiting.is_finished());

        // Dismissal frees one stack position and then one queued producer.
        popup.handle_key_event(&crossterm::event::KeyCode::Esc.into());
        let message = popup.next_message().await.unwrap();
        popup.accept(message);
        let delivered = tokio::time::timeout(Duration::from_secs(1), waiting)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(popup.content.len(), CAPACITY);
        let blocked = tokio::spawn(async {
            (Confirm::title("blocked at shutdown".into())
                .build_and_send_async()
                .await,)
        });
        tokio::task::yield_now().await;
        assert!(!blocked.is_finished());

        let (local_reply, messages) =
            with_local_messages(|| Confirm::title("local".into()).build_and_send());
        assert_eq!(messages.len(), 1);
        for message in messages {
            popup.accept(message);
        }
        assert!(popup.check());

        // An independently completed callback never has to consume its own full queue.
        popup.begin_shutdown();
        assert!(local_reply.await.is_err());
        assert!(delivered.0.await.is_err());
        let rejected = tokio::time::timeout(Duration::from_secs(1), blocked)
            .await
            .unwrap()
            .unwrap();
        assert!(rejected.0.await.is_err());
        assert!(!popup.check());
        assert!(popup.next_message().await.is_none());
    }
}
