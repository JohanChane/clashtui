#[cfg(feature = "tui")]
pub use clashctl::{
    define_enum,
    utils::{
        get_modify_time, CfgError,  SharedBackend, SharedState, State,
    },
};
pub use clashctl::{
    utils::{init_config, ClashBackend, Flag, Flags,ClashSrvOp},
    VERSION,
};
