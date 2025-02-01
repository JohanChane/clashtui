use super::{BackEnd, CallBack, ProfileManager, State};

use crate::tui::Call;
use crate::utils::consts::err as consts_err;
use tokio::sync::mpsc::{Receiver, Sender};

impl BackEnd {
    /// read file by lines, from `total_len-start-length` to `total_len-start`
    pub fn logcat(&self, start: usize, length: usize) -> anyhow::Result<Vec<String>> {
        use crate::{utils::consts::LOG_FILE, HOME_DIR};
        use std::io::BufRead as _;
        use std::io::Seek as _;
        let mut fp = std::fs::File::open(HOME_DIR.join(LOG_FILE))?;
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
    /// async runtime entry
    ///
    /// using [`tokio::sync::mpsc`] to exchange data and command
    pub async fn run(
        self,
        tx: Sender<CallBack>,
        mut rx: Receiver<Call>,
    ) -> anyhow::Result<ProfileManager> {
        use crate::tui::tabs;
        let mut errs = vec![];
        loop {
            // this blocks until recv
            let op = rx
                .recv()
                .await
                .ok_or(anyhow::anyhow!("{}", consts_err::APP_TX))?;
            let mut cbs = Vec::with_capacity(3);
            // match Tick in advance
            // DO NEVER return [`CallBack::Error`] here,
            // otherwise frontend might be 'blocked' by error messages
            if let Call::Tick = op {
                let state = match self.update_state(None, None) {
                    Ok(v) => CallBack::State(v.to_string()),
                    Err(e) => {
                        if !errs.contains(&e.to_string()) {
                            //ensure writing only once
                            log::error!("An error happens in Tick:{e}");
                            errs.push(e.to_string());
                        }
                        CallBack::State(State::unknown(self.get_current_profile().name).to_string())
                    }
                };
                cbs.push(state);
                #[cfg(feature = "connection-tab")]
                let conns = match self.api.get_connections() {
                    Ok(v) => CallBack::ConnctionInit(v),
                    Err(e) => {
                        if !errs.contains(&e.to_string()) {
                            //ensure writing only once
                            log::error!("An error happens in Tick:{e}");
                            errs.push(e.to_string());
                        }
                        CallBack::ConnctionInit(Default::default())
                    }
                };
                #[cfg(feature = "connection-tab")]
                cbs.push(conns);
            } else {
                log::debug!("Backend got:{op:?}");
                let cb = match op {
                    Call::Profile(op) => match op {
                        tabs::profile::BackendOp::Profile(op) => self.handle_profile_op(op).into(),
                        #[cfg(feature = "template")]
                        tabs::profile::BackendOp::Template(op) => {
                            self.handle_template_op(op).into()
                        }
                    },
                    Call::Service(op) => {
                        match op {
                            tabs::service::BackendOp::SwitchMode(mode) => {
                                // tick will refresh state
                                match self.update_state(None, Some(mode)) {
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
                            tabs::service::BackendOp::TuiExtend(extend_op) => match extend_op {
                                tabs::service::ExtendOp::FullLog => match self.logcat(0, 1024) {
                                    Ok(v) => CallBack::TuiExtend(v),
                                    Err(e) => CallBack::Error(e.to_string()),
                                },
                                tabs::service::ExtendOp::OpenClashtuiConfig => {
                                    if let Err(e) = crate::utils::ipc::spawn(
                                        if cfg!(windows) { "start" } else { "open" },
                                        vec![crate::HOME_DIR.to_str().unwrap()],
                                    ) {
                                        CallBack::TuiExtend(vec![
                                            "Failed".to_owned(),
                                            e.to_string(),
                                        ])
                                    } else {
                                        CallBack::TuiExtend(vec!["Success".to_owned()])
                                    }
                                }
                                tabs::service::ExtendOp::GenerateInfoList => {
                                    let mut infos = vec![
                                        "# CLASHTUI".to_owned(),
                                        format!("version:{}", crate::utils::consts::VERSION),
                                    ];
                                    infos.push("# CLASH".to_owned());
                                    match self.api.version().map_err(|e| e.into()).and_then(|ver| {
                                        self.api.config_get().map(|cfg| {
                                            let mut cfg = cfg.build();
                                            cfg.insert(2, format!("version:{ver}"));
                                            cfg
                                        })
                                    }) {
                                        Ok(info) => {
                                            infos.extend(info);
                                            CallBack::TuiExtend(infos)
                                        }
                                        Err(e) => {
                                            infos.push(format!("{e}"));
                                            CallBack::TuiExtend(infos)
                                        }
                                    }
                                }
                            },
                        }
                    }
                    #[cfg(feature = "connection-tab")]
                    Call::Connection(op) => match op {
                        tabs::connection::BackendOp::Terminal(id) => {
                            match self.api.terminate_connection(Some(id)) {
                                Ok(v) => CallBack::ConnctionCTL(if v {
                                    "Success".to_owned()
                                } else {
                                    "Failed, log as debug level".to_owned()
                                }),
                                Err(e) => CallBack::Error(e.to_string()),
                            }
                        }
                        tabs::connection::BackendOp::TerminalAll => {
                            match self.api.terminate_connection(None) {
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
                    // unfortunately, this might(in fact almost always) blocked by
                    // thousand of Call::Tick,
                    //
                    // another match might help
                    Call::Stop => return Ok(self.save()),
                    Call::Tick => unreachable!("Done in another"),
                };
                cbs.push(cb);
            }
            // send all cached package
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
                                _ => panic!("leftover value in backend rx {op:?}"),
                            });
                            Ok(self.save())
                        }
                        Some(op) => panic!("a leftover value in backend rx {op:?}"),
                        None => Err(anyhow::anyhow!("{}", consts_err::APP_RX)),
                    };
                }
            }
        }
    }
}
