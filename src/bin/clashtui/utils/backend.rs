use super::consts::err as consts_err;
use super::{
    config::{BuildConfig, ConfigFile, DataFile},
    state::State,
};
use crate::tui::Call;
use clashtui::{
    backend::{config::LibConfig, ClashBackend, ServiceOp},
    profile::{map::ProfileManager, LocalProfile, Profile},
    webapi::{ClashConfig, ClashUtil},
};
use std::path::PathBuf;
use tokio::sync::mpsc::{Receiver, Sender};

pub enum CallBack {
    Error(String),
    State(String),
    ServiceCTL(String),
}

/// a wrapper for [`ClashBackend`]
///
/// impl some other functions
pub struct BackEnd {
    inner: ClashBackend,
    profile_path: PathBuf,
    _template_path: PathBuf,
    /// just clone and merge, DO NEVER sync_to_disk/sync_from_disk
    base_profile: LocalProfile,
}
impl BackEnd {
    /// async runtime entry
    /// 
    /// use [`tokio::sync::mpsc`] to exchange data and command
    pub async fn run(self, tx: Sender<CallBack>, mut rx: Receiver<Call>) -> anyhow::Result<()> {
        use crate::tui::tabs;
        loop {
            // this blocks until recv
            let op = rx.recv().await.expect(consts_err::APP_TX);
            match op {
                Call::Service(op) => match op {
                    tabs::service::BackendOp::SwitchMode(mode) => {
                        match self.update_state(None, Some(mode.to_string())) {
                            Ok(v) => tx.send(CallBack::State(v.to_string())),
                            Err(e) => {
                                // ensure there is a refresh
                                tx.send(CallBack::State(State::unknown(
                                    self.get_current_profile().name,
                                )))
                                .await
                                .expect(consts_err::APP_RX);
                                tx.send(CallBack::Error(e.to_string()))
                            }
                        }
                    }
                    tabs::service::BackendOp::ServiceCTL(op) => match self.clash_srv_ctl(op) {
                        Ok(v) => tx.send(CallBack::ServiceCTL(v)),
                        Err(e) => tx.send(CallBack::Error(e.to_string())),
                    },
                }
                .await
                .expect(consts_err::APP_RX),

                Call::Stop => return Ok(()),
                // register some real-time work here
                //
                // DO NECER return [`CallBack::Error`], 
                // otherwise tui might be 'blocked' by error message
                Call::Tick => match self.update_state(None, None) {
                    Ok(v) => tx.send(CallBack::State(v.to_string())),
                    Err(e) => {
                        tx.send(CallBack::State(State::unknown(
                            self.get_current_profile().name,
                        )))
                        //write this direct to log, write only once
                    }
                }
                .await
                .expect(consts_err::APP_RX),
            }
        }
    }
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

impl BackEnd {
    pub fn clash_srv_ctl(&self, op: ServiceOp) -> std::io::Result<String> {
        self.inner.clash_srv_ctl(op)
    }
    pub fn restart_clash(&self) -> Result<String, String> {
        self.inner.api.restart(None).map_err(|e| e.to_string())
    }
    pub fn update_state(
        &self,
        new_pf: Option<String>,
        new_mode: Option<String>,
    ) -> anyhow::Result<State> {
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
        let sysp = self.inner.is_system_proxy_enabled().map_or_else(
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

impl TryFrom<BuildConfig> for BackEnd {
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
