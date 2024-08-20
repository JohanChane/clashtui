use super::{
    config::{BuildConfig, ConfigFile, DataFile},
    state::State,
};
use clashtui::{
    backend::{config::LibConfig, ClashBackend, ClashSrvOp},
    profile::{map::ProfileManager, LocalProfile, Profile},
    webapi::{ClashConfig, ClashUtil},
};
use std::path::PathBuf;

/// a wrapper for [`ClashBackend`]
/// 
/// impl some other functions
pub struct Backend {
    inner: ClashBackend,
    profile_path: PathBuf,
    _template_path: PathBuf,
    /// just clone and merge, DO NEVER sync_to_disk/sync_from_disk
    base_profile: LocalProfile,
}

impl Backend {
    pub fn get_all_profiles(&self) -> Vec<Profile> {
        self.inner.get_all_profiles()
    }
    pub fn get_current_profile(&self) -> Profile {
        self.inner.get_current_profile().unwrap_or_default()
    }
    pub fn update_profile(
        &self,
        profile: &Profile,
        update_all: bool,
        with_proxy: Option<bool>,
    ) -> anyhow::Result<Vec<String>> {
        let path = self.profile_path.join(&profile.name);
        let profile = self.inner.load_local_profile(profile, path)?;
        self.inner.update_profile(&profile, update_all, with_proxy)
    }

    pub fn select_profile(&self, profile: Profile) -> anyhow::Result<()> {
        // load selected profile
        let path = self.profile_path.join(&profile.name);
        let lprofile = self.inner.load_local_profile(&profile, path)?;
        // merge that into basic profile
        let mut new_profile = self.base_profile.clone();
        new_profile.merge(&lprofile)?;
        // set path to clash config file path and sync to disk
        new_profile.path = self.inner.cfg.basic.clash_cfg_pth.clone().into();
        new_profile.sync_to_disk()?;
        // after, change current profile
        self.inner.set_current_profile(profile);
        // ask clash to reload config
        self.inner
            .api
            .config_reload(&self.inner.cfg.basic.clash_cfg_pth)?;
        Ok(())
    }
}

impl Backend {
    pub fn clash_srv_ctl(&self, op: ClashSrvOp) -> std::io::Result<String> {
        self.inner.clash_srv_ctl(op)
    }
    pub fn restart_clash(&self) -> Result<String, String> {
        self.inner.api.restart(None).map_err(|e| e.to_string())
    }
    pub fn update_state(
        &self,
        new_pf: Option<String>,
        new_mode: Option<String>,
        #[cfg(target_os = "windows")] new_sysp: Option<bool>,
    ) -> anyhow::Result<State> {
        #[cfg(target_os = "windows")]
        use crate::utils::ipc;
        #[cfg(target_os = "windows")]
        if let Some(b) = new_sysp {
            let _ = if b {
                ipc::enable_system_proxy(&self.clash_api.proxy_addr)
            } else {
                ipc::disable_system_proxy()
            };
        }
        if let Some(mode) = new_mode {
            self.inner.update_mode(mode)?;
        }
        if let Some(pf) = new_pf.as_ref() {
            if let Some(pf) = self.inner.get_profile(pf) {
                self.select_profile(pf)?;
            } else {
                anyhow::bail!("Not a recorded profile");
            };
        }
        #[cfg(target_os = "windows")]
        let sysp = ipc::is_system_proxy_enabled().map_or_else(
            |v| {
                log::error!("{}", v);
                None
            },
            Some,
        );
        let ClashConfig { mode, tun, .. } = self.inner.api.config_get()?;
        Ok(State {
            profile: new_pf.unwrap_or(self.get_current_profile().name),
            mode: Some(mode),
            tun: Some(tun.stack),
            #[cfg(target_os = "windows")]
            sysproxy: sysp,
        })
    }
}

impl TryFrom<BuildConfig> for Backend {
    type Error = anyhow::Error;
    fn try_from(value: BuildConfig) -> Result<Self, Self::Error> {
        let BuildConfig {
            cfg,
            basic: info,
            data,
            profile_dir: profile_path,
            template_dir: template_path,
            base_raw,
        } = value;
        let ConfigFile {
            basic,
            service,
            timeout,
        } = cfg;
        let cfg = LibConfig { basic, service };
        let (external_controller, proxy_addr, secret, global_ua) = info.build()?;
        let api = ClashUtil::new(external_controller, secret, proxy_addr, global_ua, timeout);
        let DataFile {
            profiles,
            current_profile,
        } = data;
        let pm = ProfileManager::new(current_profile, profiles);
        let base_profile = LocalProfile {
            content: Some(base_raw),
            ..LocalProfile::default()
        };
        Ok(Self {
            inner: ClashBackend { api, cfg, pm },
            profile_path,
            _template_path: template_path,
            base_profile,
        })
    }
}
