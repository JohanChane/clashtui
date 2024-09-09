pub mod config;
mod impl_profile;
mod impl_service;
#[cfg(feature = "template")]
mod impl_template;
mod impl_webapi;
pub mod ipc;
pub(super) mod util;

pub use impl_service::ServiceOp;

use super::backend::config::LibConfig;
use super::profile::map::ProfileManager;
use super::webapi::ClashUtil;

pub struct ClashBackend {
    pub api: ClashUtil,
    pub cfg: LibConfig,
    pub pm: ProfileManager,
}
