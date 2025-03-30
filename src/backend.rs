mod impl_profile;
mod impl_service;
#[cfg(feature = "template")]
mod impl_template;
#[cfg(feature = "tui")]
mod impl_tui;

use crate::clash::webapi::ClashUtil;
// configs
use crate::utils::config::{BuildConfig, LibConfig};
// ipc
use crate::utils::ipc;
// profile
pub use impl_profile::ProfileType;
pub use impl_profile::{database::ProfileManager, Profile};
// service
pub use impl_service::ServiceOp;
use impl_service::State;

#[cfg(feature = "tui")]
#[derive(derive_more::Debug)]
pub enum CallBack {
    Error(String),
    State(String),
    Logs(Vec<String>),
    TuiExtend(String),
    Edit,
    Preview(Vec<String>),
    ServiceCTL(String),

    ProfileInit(Vec<String>, Vec<Option<core::time::Duration>>),
    ProfileCTL(Vec<String>),

    #[cfg(feature = "connection-tab")]
    ConnectionInit(#[debug(skip)] crate::clash::webapi::ConnInfo),
    #[cfg(feature = "connection-tab")]
    ConnectionCTL(String),

    #[cfg(feature = "template")]
    TemplateInit(Vec<String>),
    #[cfg(feature = "template")]
    TemplateCTL(Vec<String>),
}

#[cfg(feature = "tui")]
impl From<anyhow::Result<CallBack>> for CallBack {
    fn from(value: anyhow::Result<CallBack>) -> Self {
        match value {
            Ok(v) => v,
            Err(e) => CallBack::Error(e.to_string()),
        }
    }
}

pub struct BackEnd {
    api: ClashUtil,
    cfg: LibConfig,
    pm: ProfileManager,
    edit_cmd: String,
    open_dir_cmd: String,
    /// This is `basic_clash_config.yaml` in memory
    base_profile: serde_yml::Mapping,
}

impl BackEnd {
    pub fn build(value: BuildConfig) -> Self {
        let BuildConfig {
            cfg,
            data,
            base_profile: base_raw,
            edit_cmd,
            open_dir_cmd,
            timeout,
            external_controller,
            proxy_addr,
            secret,
            global_ua,
        } = value;
        crate::clash::webapi::set_timeout(timeout);
        let api = ClashUtil::new(external_controller, secret, proxy_addr, global_ua);
        Self {
            api,
            cfg,
            pm: data,
            edit_cmd,
            base_profile: base_raw,
            open_dir_cmd,
        }
    }
    pub fn get_config(&self) -> &LibConfig {
        &self.cfg
    }
    /// Save all in-memory data to file
    fn save(self) -> ProfileManager {
        self.pm
    }
}
