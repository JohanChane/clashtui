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
    Logs(Vec<String>),
    ServiceCTL(String),
    ProfileCTL(Vec<String>),
    ProfileInit(Vec<String>, Vec<Option<core::time::Duration>>),
    #[cfg(feature = "template")]
    TemplateInit(Vec<String>),
}
impl std::fmt::Display for CallBack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                CallBack::Error(_) => "Error",
                CallBack::State(_) => "State",
                CallBack::Logs(_) => "Logs",
                CallBack::ServiceCTL(_) => "ServiceCTL",
                CallBack::ProfileCTL(_) => "ProfileCTL",
                CallBack::ProfileInit(_, _) => "ProfileInit",
                #[cfg(feature = "template")]
                CallBack::TemplateInit(_) => "TemplateInit",
            }
        )
    }
}

/// a wrapper for [`ClashBackend`]
///
/// impl some other functions
pub struct BackEnd {
    inner: ClashBackend,
    log_file: PathBuf,
    profile_path: PathBuf,
    #[cfg(feature = "template")]
    template_path: PathBuf,
    /// just clone and merge, DO NEVER sync_to_disk/sync_from_disk
    base_profile: LocalProfile,
}
impl BackEnd {
    /// async runtime entry
    ///
    /// use [`tokio::sync::mpsc`] to exchange data and command
    pub async fn run(
        self,
        tx: Sender<CallBack>,
        mut rx: Receiver<Call>,
    ) -> anyhow::Result<DataFile> {
        use crate::tui::tabs;
        let mut errs = vec![];
        loop {
            // this blocks until recv
            let op = rx
                .recv()
                .await
                .ok_or(anyhow::anyhow!("{}", consts_err::APP_TX))?;
            let cb = match op {
                Call::Profile(op) => match op {
                    tabs::profile::BackendOp::Profile(op) => match op {
                        tabs::profile::ProfileOp::GetALL => CallBack::ProfileInit(
                            self.get_all_profiles()
                                .into_iter()
                                .map(|p| p.name)
                                .collect(),
                            self.get_all_profiles()
                                .into_iter()
                                .map(|p| self.load_local_profile(&p).ok())
                                .map(|lp| lp.and_then(|lp| lp.atime()))
                                .collect(),
                        ),
                        tabs::profile::ProfileOp::Add(name, url) => {
                            self.create_profile(&name, url);
                            match self.update_profile(
                                &self
                                    .get_profile(name)
                                    .expect("Cannot find selected profile"),
                                None,
                            ) {
                                Ok(v) => CallBack::ProfileCTL(v),
                                Err(e) => CallBack::Error(e.to_string()),
                            }
                        }
                        tabs::profile::ProfileOp::Remove(name) => {
                            if let Err(e) = self.remove_profile(
                                self.get_profile(name)
                                    .expect("Cannot find selected profile"),
                            ) {
                                CallBack::Error(e.to_string())
                            } else {
                                CallBack::ProfileCTL(vec!["Profile is now removed".to_owned()])
                            }
                        }
                        tabs::profile::ProfileOp::Update(name, with_proxy) => {
                            match self.update_profile(
                                &self
                                    .get_profile(name)
                                    .expect("Cannot find selected profile"),
                                with_proxy,
                            ) {
                                Ok(v) => CallBack::ProfileCTL(v),
                                Err(e) => CallBack::Error(e.to_string()),
                            }
                        }
                        tabs::profile::ProfileOp::Select(name) => {
                            if let Err(e) = self.select_profile(
                                self.get_profile(name)
                                    .expect("Cannot find selected profile"),
                            ) {
                                CallBack::Error(e.to_string())
                            } else {
                                CallBack::ProfileCTL(vec!["Profile is now loaded".to_owned()])
                            }
                        }
                    },
                    #[cfg(feature = "template")]
                    tabs::profile::BackendOp::Template(op) => match op {
                        tabs::profile::TemplateOp::GetALL => {
                            CallBack::TemplateInit(self.get_all_templates())
                        }
                        tabs::profile::TemplateOp::Add(_) => todo!(),
                        tabs::profile::TemplateOp::Remove(_) => todo!(),
                        tabs::profile::TemplateOp::Generate(_) => todo!(),
                    },
                },
                Call::Service(op) => {
                    match op {
                        tabs::service::BackendOp::SwitchMode(mode) => {
                            // tick will refresh state
                            match self.update_state(None, Some(mode.to_string())) {
                                Ok(_) => {
                                    CallBack::ServiceCTL(format!("Mode is switched to {}", mode))
                                }
                                Err(e) => CallBack::Error(e.to_string()),
                            }
                        }
                        tabs::service::BackendOp::ServiceCTL(op) => match self.clash_srv_ctl(op) {
                            Ok(v) => CallBack::ServiceCTL(v),
                            Err(e) => CallBack::Error(e.to_string()),
                        },
                    }
                }
                Call::Logs(start, len) => match self.logcat(start, len) {
                    Ok(v) => CallBack::Logs(v),
                    Err(e) => CallBack::Error(e.to_string()),
                },
                // unfortunately, this might(in facmatches!(rx.recv().await.unwrap(), Call::Stop)t almost always) block by
                // thousand of [Call::Tick],
                //
                // another match might help
                Call::Stop => return Ok(self.save()),
                // register some real-time work here
                //
                // DO NEVER return [`CallBack::Error`],
                // otherwise tui might be 'blocked' by error message
                Call::Tick => {
                    match self.update_state(None, None) {
                        Ok(v) => CallBack::State(v.to_string()),
                        Err(e) => {
                            if !errs.contains(&e.to_string()) {
                                log::error!("An error happens in Tick:{e}");
                                errs.push(e.to_string());
                            }
                            CallBack::State(State::unknown(self.get_current_profile().name))
                            //write this direct to log, write only once
                        }
                    }
                }
            };
            if let Err(_) = tx.send(cb).await {
                return match rx.recv().await {
                    // normal shutdown
                    Some(Call::Stop) => Ok(self.save()),
                    // try match other op in channel if there is
                    //
                    // I use panic in hope to catch those at develop time
                    Some(Call::Tick) => {
                        let mut buf = vec![];
                        rx.recv_many(&mut buf, 10).await;
                        buf.into_iter().for_each(|op| match op {
                            Call::Tick | Call::Stop => {}
                            _ => panic!("leftover value in backend rx {op}"),
                        });
                        Ok(self.save())
                    }
                    Some(op) => panic!("a leftover value in backend rx {op}"),
                    None => Err(anyhow::anyhow!("{}", consts_err::APP_RX)),
                };
            }
        }
    }
    fn save(&self) -> DataFile {
        let (current_profile, profiles) = self.inner.pm.clone_inner();
        let data = DataFile {
            profiles,
            current_profile,
        };
        data
    }
}
impl BackEnd {
    /// read file by lines, from `total_len-start-length` to `total_len-start`
    pub fn logcat(&self, start: usize, length: usize) -> anyhow::Result<Vec<String>> {
        use std::io::BufRead as _;
        use std::io::Seek as _;
        let mut fp = std::fs::File::open(&self.log_file)?;
        let size = {
            let fp = fp.try_clone()?;
            std::io::BufReader::new(fp).lines().count()
        };
        fp.seek(std::io::SeekFrom::Start(0))?;
        let fp = std::io::BufReader::new(fp).lines();
        let start = (size as usize).checked_sub(start + length).unwrap_or(0);
        let vec = fp
            .skip(start)
            .take(length)
            .collect::<std::io::Result<_>>()?;
        Ok(vec)
    }
}

impl BackEnd {
    fn create_profile<S: AsRef<str>, S2: AsRef<str>>(&self, name: S, url: S2) {
        self.inner.pm.insert(
            name,
            clashtui::profile::ProfileType::Url(url.as_ref().to_owned()),
        );
    }
    fn remove_profile(&self, pf: Profile) -> anyhow::Result<()> {
        let LocalProfile { path, .. } = self.load_local_profile(&pf)?;
        Ok(std::fs::remove_file(path)?)
    }
    fn get_profile<S: AsRef<str>>(&self, name: S) -> Option<Profile> {
        self.inner.get_profile(name)
    }
    pub fn get_all_profiles(&self) -> Vec<Profile> {
        self.inner.get_all_profiles()
    }
    pub fn get_current_profile(&self) -> Profile {
        self.inner.get_current_profile().unwrap_or_default()
    }
    fn load_local_profile(&self, pf: &Profile) -> anyhow::Result<LocalProfile> {
        let path = self.profile_path.join(&pf.name);
        self.inner.load_local_profile(pf, path)
    }
    // TODO: plan to treat None as auto
    pub fn update_profile(
        &self,
        profile: &Profile,
        with_proxy: Option<bool>,
    ) -> anyhow::Result<Vec<String>> {
        let profile = self.load_local_profile(profile)?;
        self.inner.update_profile(&profile, with_proxy)
    }

    pub fn select_profile(&self, profile: Profile) -> anyhow::Result<()> {
        // load selected profile
        let lprofile = self.load_local_profile(&profile)?;
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

#[cfg(feature = "template")]
impl BackEnd {
    pub fn get_all_templates(&self) -> Vec<String> {
        self.inner.get_all_templates()
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

impl BackEnd {
    pub fn build(value: BuildConfig, log_file: PathBuf) -> Result<Self, anyhow::Error> {
        let BuildConfig {
            cfg,
            basic: info,
            data,
            profile_dir: profile_path,
            template_dir: _template_path,
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
            log_file,
            #[cfg(feature = "template")]
            template_path: _template_path,
            base_profile,
        })
    }
}
