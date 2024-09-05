mod impl_profile;
mod impl_service;
#[cfg(feature = "template")]
mod impl_template;

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
use tokio::sync::mpsc::{Receiver, Sender};

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
    ConnctionInit(clashtui::webapi::ConnInfo),
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
                CallBack::Infos(_) => "Infos",
                CallBack::Edit => "Edit",
                CallBack::Preview(_) => "Preview",
                CallBack::ServiceCTL(_) => "ServiceCTL",
                CallBack::ProfileCTL(_) => "ProfileCTL",
                CallBack::ProfileInit(..) => "ProfileInit",
                #[cfg(feature = "template")]
                CallBack::TemplateInit(_) => "TemplateInit",
                #[cfg(feature = "connection-tab")]
                CallBack::ConnctionCTL(_) => "ConnctionTab",
                #[cfg(feature = "connection-tab")]
                CallBack::ConnctionInit(_) => "ConnctionInit",
            }
        )
    }
}

/// a wrapper for [`ClashBackend`]
///
/// impl some other functions
pub struct BackEnd {
    inner: ClashBackend,
    edit_cmd: String,
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
            let mut cbs = Vec::with_capacity(2);
            if let Call::Tick = op {
                // register some real-time work here
                //
                // DO NEVER return [`CallBack::Error`],
                // otherwise tui might be 'blocked' by error message
                let state = match self.update_state(None, None) {
                    // TODO: add connection-tab update return here
                    Ok(v) => CallBack::State(v.to_string()),
                    Err(e) => {
                        if !errs.contains(&e.to_string()) {
                            log::error!("An error happens in Tick:{e}");
                            errs.push(e.to_string());
                        }
                        CallBack::State(State::unknown(self.get_current_profile().name))
                        //write this direct to log, write only once
                    }
                };
                cbs.push(state);
                #[cfg(feature = "connection-tab")]
                let conns = match self.inner.api.get_connections() {
                    Ok(v) => CallBack::ConnctionInit(v),
                    Err(e) => {
                        if !errs.contains(&e.to_string()) {
                            log::error!("An error happens in Tick:{e}");
                            errs.push(e.to_string());
                        }
                        CallBack::ConnctionInit(Default::default())
                        //write this direct to log, write only once
                    }
                };
                #[cfg(feature = "connection-tab")]
                cbs.push(conns);
            } else {
                let cb = match op {
                    Call::Profile(op) => match op {
                        tabs::profile::BackendOp::Profile(op) => self.handle_profile_op(op),
                        #[cfg(feature = "template")]
                        tabs::profile::BackendOp::Template(op) => self.handle_template_op(op),
                    },
                    Call::Service(op) => {
                        match op {
                            tabs::service::BackendOp::SwitchMode(mode) => {
                                // tick will refresh state
                                match self.update_state(None, Some(mode.to_string())) {
                                    Ok(_) => CallBack::ServiceCTL(format!(
                                        "Mode is switched to {}",
                                        mode
                                    )),
                                    Err(e) => CallBack::Error(e.to_string()),
                                }
                            }
                            tabs::service::BackendOp::ServiceCTL(op) => {
                                match self.clash_srv_ctl(op) {
                                    Ok(v) => CallBack::ServiceCTL(v),
                                    Err(e) => CallBack::Error(e.to_string()),
                                }
                            }
                        }
                    }
                    #[cfg(feature = "connection-tab")]
                    Call::Connection(op) => match op {
                        tabs::connection::BackendOp::Terminal(id) => {
                            match self.inner.api.terminate_connection(Some(id)) {
                                Ok(v) => CallBack::ConnctionCTL(if v {
                                    "Success".to_owned()
                                } else {
                                    "Failed, log as debug level".to_owned()
                                }),
                                Err(e) => CallBack::Error(e.to_string()),
                            }
                        }
                        tabs::connection::BackendOp::TerminalAll => {
                            match self.inner.api.terminate_connection(None) {
                                Ok(v) => CallBack::ConnctionCTL(if v {
                                    "Success".to_owned()
                                } else {
                                    "Failed, log as debug level".to_owned()
                                }),
                                Err(e) => CallBack::Error(e.to_string()),
                            }
                        }
                    },
                    Call::Logs(start, len) => match self.logcat(start, len) {
                        Ok(v) => CallBack::Logs(v),
                        Err(e) => CallBack::Error(e.to_string()),
                    },
                    Call::Infos => {
                        let mut infos = vec![
                            "# CLASHTUI".to_owned(),
                            format!("version:{}", crate::utils::consts::VERSION),
                        ];
                        match self
                            .inner
                            .api
                            .version()
                            .map_err(|e| e.into())
                            .and_then(|ver| {
                                self.inner.api.config_get().map(|cfg| {
                                    let mut cfg = cfg.build();
                                    cfg.insert(2, "# CLASH".to_owned());
                                    cfg.insert(3, format!("version:{ver}"));
                                    cfg
                                })
                            }) {
                            Ok(info) => {
                                infos.extend(info);
                                CallBack::Infos(infos)
                            }
                            Err(e) => {
                                infos.extend(["# CLASH".to_owned(), format!("{e}")]);
                                CallBack::Infos(infos)
                            }
                        }
                    }
                    // unfortunately, this might(in fact almost always) block by
                    // thousand of [Call::Tick],
                    //
                    // another match might help
                    Call::Stop => return Ok(self.save()),
                    Call::Tick => unreachable!("Done in another"),
                };
                cbs.push(cb);
            }
            //
            for cb in cbs {
                if tx.send(cb).await.is_err() {
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
    }
    fn save(&self) -> DataFile {
        let (current_profile, profiles) = self.inner.pm.clone_inner();
        DataFile {
            profiles,
            current_profile,
        }
    }
}

impl BackEnd {
    /// read file by lines, from `total_len-start-length` to `total_len-start`
    pub fn logcat(&self, start: usize, length: usize) -> anyhow::Result<Vec<String>> {
        use crate::{utils::consts::LOG_FILE, HOME_DIR};
        use std::io::BufRead as _;
        use std::io::Seek as _;
        let mut fp = std::fs::File::open(HOME_DIR.get().unwrap().join(LOG_FILE))?;
        let size = {
            let fp = fp.try_clone()?;
            std::io::BufReader::new(fp).lines().count()
        };
        fp.seek(std::io::SeekFrom::Start(0))?;
        let fp = std::io::BufReader::new(fp).lines();
        let start = size.saturating_sub(start + length);
        let vec = fp
            .skip(start)
            .take(length)
            .collect::<std::io::Result<_>>()?;
        Ok(vec)
    }
}

impl BackEnd {
    pub fn build(value: BuildConfig) -> Result<Self, anyhow::Error> {
        let BuildConfig {
            cfg,
            basic: info,
            data,
            base_raw,
        } = value;
        let ConfigFile {
            basic,
            service,
            timeout,
            edit_cmd,
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
            edit_cmd,
            base_profile,
        })
    }
}
