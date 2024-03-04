mod config;
mod flags;
mod ipc;
mod state;
mod tui;
mod impl_app;
mod impl_profile;
mod impl_clashsrv;
#[allow(clippy::module_inception)]
mod utils;

pub type SharedClashTuiUtil = std::rc::Rc<tui::ClashTuiUtil>;
pub type SharedClashTuiState = std::rc::Rc<std::cell::RefCell<State>>;

pub use config::{init_config, CfgError, ErrKind};
pub use flags::{Flag, Flags};
pub use state::State;
pub use tui::ClashTuiUtil;
pub use utils::*;

