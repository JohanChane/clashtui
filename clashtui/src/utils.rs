pub use backend::ClashBackend;
pub use backend::{
    api,
    utils::{init_config, ClashSrvOp},
};
mod flags;
pub use flags::{BitFlags as Flags, Flag};

pub(crate) const VERSION: &str = concat!(env!("CLASHTUI_VERSION"));

#[cfg(feature = "tui")]
pub use backend::{
    define_enum,
    utils::{get_modify_time, CfgError, State as _State},
};
#[cfg(feature = "tui")]
mod state;
#[cfg(feature = "tui")]
pub use state::State;

#[cfg(feature = "tui")]
pub type SharedState = std::rc::Rc<core::cell::RefCell<State>>;
#[cfg(feature = "tui")]
pub type SharedBackend = std::rc::Rc<ClashBackend>;
