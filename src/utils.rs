// mod backend;
pub mod config;
pub mod ipc;
mod macros;
pub mod profile;
pub mod self_update;
pub mod state;

// #[cfg(feature = "tui")]
// pub(crate) use backend::CallBack;
// pub(crate) use backend::{BackEnd, ServiceOp};
pub(crate) mod consts;

pub(crate) use config::BuildConfig;
pub(crate) use profile::Profile;
