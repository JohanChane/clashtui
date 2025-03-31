mod browser;
mod list;
mod popup;
pub mod tools;

pub use browser::{Browser, Path};
pub use list::List;
pub use popup::{Popmsg, Popup, PopupState};

pub struct PopMsg(Box<dyn Popmsg>);

impl PopMsg {
    pub fn new(f: impl Popmsg + 'static) -> Self {
        Self(Box::new(f))
    }
    pub fn working() -> Self {
        Self(Box::new(popup::Working))
    }
    pub fn msg(msg: String) -> Self {
        Self(Box::new(popup::Msg(msg)))
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum PopRes {
    /// result of [PopMsg::AskChoices]
    // Choices(usize),
    /// result of [PopMsg::SelectList]
    Selected(usize),
    SelectedMulti(Vec<usize>),
    /// result of [PopMsg::Input]
    Input(String),
}
