pub(crate) mod input;
// mod prompt;

mod dev {
    pub use crate::tui::widget::popmsg::{Msg, MsgBuilder, Route};
    pub use crate::tui::Key;
    pub use crossterm::event::KeyCode;
    pub use ratatui::prelude::{Frame, Rect};
    pub use ratatui::widgets::Block;
    pub use tokio::sync::oneshot::Sender;
}

pub mod prelude {
    pub use super::input::Input;
    pub use super::input::InputMasked;
    // pub use super::prompt::Prompt;
}
