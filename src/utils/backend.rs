mod impl_profile;
mod impl_service;
#[cfg(feature = "template")]
mod impl_template;
#[cfg(feature = "tui")]
mod impl_tui;

#[cfg(feature = "tui")]
#[derive(derive_more::Debug)]
pub enum CallBack {
    Error(String),
    State(String),
    Logs(Vec<String>),
    Infos(Vec<String>),
    Edit,
    Preview(Vec<String>),
    ServiceCTL(String),
    ProfileCTL(Vec<String>),
    #[cfg(feature = "connection-tab")]
    ConnctionCTL(String),
    #[cfg(feature = "connection-tab")]
    ConnctionInit(#[debug(skip)] crate::clash::webapi::ConnInfo),
    ProfileInit(Vec<String>, Vec<Option<core::time::Duration>>),
    #[cfg(feature = "template")]
    TemplateInit(Vec<String>),
}

use super::{
    config::{BuildConfig, ConfigFile, DataFile},
    ipc,
    state::State,
};
use crate::clash::{
    config::LibConfig,
    profile::{map::ProfileManager, LocalProfile, Profile},
    webapi::{ClashConfig, ClashUtil},
};
pub use impl_service::ServiceOp;

pub struct BackEnd {
    api: ClashUtil,
    cfg: LibConfig,
    pm: ProfileManager,
    edit_cmd: String,
    /// just clone and merge, DO NEVER sync_to_disk/sync_from_disk
    base_profile: LocalProfile,
}

impl BackEnd {
    pub fn build(value: BuildConfig) -> Result<Self, anyhow::Error> {
        let BuildConfig {
            cfg:
                ConfigFile {
                    basic,
                    service,
                    timeout,
                    edit_cmd,
                },
            basic: info,
            data:
                DataFile {
                    profiles,
                    current_profile,
                },
            base_raw,
        } = value;
        let cfg = LibConfig { basic, service };
        let (external_controller, proxy_addr, secret, global_ua) = info.build()?;
        let api = ClashUtil::new(external_controller, secret, proxy_addr, global_ua, timeout);
        let pm = ProfileManager::new(current_profile, profiles);
        let base_profile = LocalProfile {
            content: Some(base_raw),
            ..LocalProfile::default()
        };
        Ok(Self {
            api,
            cfg,
            pm,
            edit_cmd,
            base_profile,
        })
    }
}
