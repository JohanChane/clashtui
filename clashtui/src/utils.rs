#[cfg(feature = "tui")]
use backend::utils::State as _State;
pub use backend::{
    api,
    utils::{init_config, ClashBackend, ClashSrvOp},
};
#[cfg(feature = "tui")]
pub use backend::{
    define_enum,
    utils::{get_modify_time, CfgError},
};
pub(crate) const VERSION: &str = concat!(env!("CLASHTUI_VERSION"));
mod flags;
#[cfg(feature = "tui")]
mod state;
pub use flags::{BitFlags as Flags, Flag};
#[cfg(feature = "tui")]
pub use state::State;

#[cfg(feature = "tui")]
pub type SharedState = std::rc::Rc<core::cell::RefCell<State>>;
#[cfg(feature = "tui")]
pub type SharedBackend = std::rc::Rc<ClashBackend>;
