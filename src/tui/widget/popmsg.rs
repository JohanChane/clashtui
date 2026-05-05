use crate::tui::Key;
use crate::tui::TuiWidget;
use ratatui::prelude::{Frame, Rect};
use ratatui::widgets::Block;
use std::sync::{LazyLock, Mutex, mpsc};
use tokio::sync::oneshot::Sender;

mod builder;
mod confirm;
mod wrapper;

pub use builder::MsgBuilder;
pub use confirm::Confirm;
use wrapper::{Prompt, Wrapped};

static PAIR: LazyLock<(mpsc::Sender<Wrapped>, Mutex<mpsc::Receiver<Wrapped>>)> =
    LazyLock::new(|| {
        let (tx, rx) = mpsc::channel();
        (tx, rx.into())
    });

pub enum Route {
    Keep,
    Send,
    Drop,
}

pub trait Msg {
    type Result;

    fn match_key_event(&mut self, kv: &Key) -> Route;
    fn send(self, tx: Sender<Self::Result>);
    fn render(&self, f: &mut Frame, area: Rect, block: Block, is_focused: bool);
    /// (Width, Height)
    fn size(&self) -> (u16, u16);
}

#[derive(Default)]
pub struct PopUp {
    content: Vec<Wrapped>,
}

impl PopUp {
    pub fn check(&mut self) -> bool {
        !self.content.is_empty()
    }
}

impl TuiWidget for PopUp {
    fn handle_key_event(&mut self, kv: &Key) {
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

    /// `area` is not needed but kept for tarit
    fn render(&mut self, f: &mut Frame, _: Rect) {
        self.content.iter_mut().for_each(|c| c.render(f));
    }

    fn sync(&mut self) {
        while let Ok(content) = PAIR.1.lock().unwrap().try_recv() {
            self.content.push(content.into());
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
