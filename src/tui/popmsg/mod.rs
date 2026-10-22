mod confirm;
mod fzf;
pub(crate) mod input;

mod dev {
    pub(super) use crate::tui::Key;
    pub use crate::tui::widget::popmsg::{Msg, MsgBuilder, Route};
    pub use crossterm::event::KeyCode;
    pub use ratatui::prelude::{Frame, Rect};
    pub use ratatui::widgets::Block;
    pub use tokio::sync::oneshot::Sender;
}

pub mod prelude {
    pub use super::confirm::Confirm;
    pub use super::fzf::FzfFinder;
    pub use super::input::Input;
}
