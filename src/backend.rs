mod clash;
mod impl_profile;
mod impl_service;
#[cfg(feature = "template")]
mod impl_template;
#[cfg(feature = "tui")]
mod impl_tui;
mod state;

use clash::webapi::ClashUtil;
#[cfg(feature = "connections")]
pub use clash::webapi::{Conn, ConnInfo, ConnMetaData};
pub use clash::{get_blob, headers, webapi::Mode};
// configs
use crate::utils::config::{Basic, BuildConfig, LibConfig};
// ipc
use crate::utils::ipc;
// profile
pub use impl_profile::ProfileType;
pub use impl_profile::{Profile, database::ProfileManager};
// service
pub use impl_service::ServiceOp;
use state::State;

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

    #[cfg(feature = "connections")]
    ConnectionInit(#[debug(skip)] clash::webapi::ConnInfo),
    #[cfg(feature = "connections")]
    ConnectionCTL(String),

    #[cfg(feature = "template")]
    TemplateInit(Vec<String>),
    #[cfg(feature = "template")]
    TemplateCTL(Vec<String>),
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
            base_profile,
            edit_cmd,
            open_dir_cmd,
            timeout,
            external_controller,
            proxy_addr,
            secret,
            global_ua,
        } = value;
        let api = ClashUtil::new(external_controller, secret, proxy_addr, global_ua, timeout);
        Self {
            api,
            cfg,
            pm: data,
            edit_cmd,
            base_profile,
            open_dir_cmd,
        }
    }
    pub fn get_mihomo_bin_path(&self) -> &str {
        &self.cfg.basic.clash_bin_path
    }
    /// Save all in-memory data to file
    fn save(self) -> ProfileManager {
        self.pm
    }
}

pub struct ProfileBackend<'a> {
    pm: &'a ProfileManager,
    api: &'a ClashUtil,
    base_profile: &'a serde_yml::Mapping,
    cfg: &'a Basic,
    edit_cmd: &'a str,
}

impl BackEnd {
    pub fn as_profile(&self) -> ProfileBackend {
        ProfileBackend {
            pm: &self.pm,
            api: &self.api,
            base_profile: &self.base_profile,
            cfg: &self.cfg.basic,
            edit_cmd: &self.edit_cmd,
        }
    }
}
