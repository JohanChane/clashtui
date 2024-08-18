pub use crate::backend::ClashBackend;
pub use crate::backend::{
    api,
    utils::{init_config, ClashSrvOp},
};
mod flags;
pub use flags::{BitFlags as Flags, Flag};
pub(crate) mod consts;

#[cfg(feature = "tui")]
pub use backend::{
    define_enum,
    utils::{get_modify_time, State as _State},
};
#[cfg(feature = "tui")]
mod state;
#[cfg(feature = "tui")]
pub use state::State;

#[cfg(feature = "tui")]
pub type SharedState = std::rc::Rc<core::cell::RefCell<State>>;
#[cfg(feature = "tui")]
pub type SharedBackend = std::rc::Rc<ClashBackend>;
