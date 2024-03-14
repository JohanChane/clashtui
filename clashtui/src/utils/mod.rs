mod config;
mod flags;
mod ipc;
mod state;
mod tui;
#[allow(clippy::module_inception)]
mod utils;

pub type SharedClashTuiUtil = std::rc::Rc<tui::ClashTuiUtil>;
pub type SharedClashTuiState = std::rc::Rc<core::cell::RefCell<State>>;

pub use config::{init_config, CfgError};
pub use flags::{Flag, BitFlags as Flags};
pub use state::State;
pub use tui::ClashTuiUtil;
pub use utils::*;
