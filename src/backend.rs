pub mod config;
mod impl_profile;
mod impl_service;
#[cfg(feature = "template")]
mod impl_template;
mod impl_webapi;
mod ipc;
pub(crate) mod util;

pub use impl_service::ServiceOp;

use crate::backend::config::LibConfig;
use crate::profile::map::ProfileManager;
use crate::webapi::ClashUtil;

pub struct ClashBackend {
    pub api: ClashUtil,
    pub cfg: LibConfig,
    pub pm: ProfileManager,
}

impl ClashBackend {
    pub fn new(cfg: LibConfig, api: ClashUtil, profiles: ProfileManager) -> Self {
        Self {
            api,
            cfg,
            pm: profiles,
        }
    }
}
