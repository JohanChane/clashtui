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
pub use flags::{BitFlags as Flags, Flag};
pub use state::State;
pub use tui::ClashTuiUtil;
pub use utils::*;
