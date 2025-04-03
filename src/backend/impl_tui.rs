use super::{BackEnd, CallBack, ProfileManager, State};

use crate::tui::Call;
use crate::utils::{consts::err as consts_err, logging::logcat};
use tokio::sync::mpsc::{Receiver, Sender};

impl<E: ToString> From<Result<CallBack, E>> for CallBack {
    fn from(value: Result<CallBack, E>) -> Self {
        match value {
            Ok(v) => v,
            Err(e) => CallBack::Error(e.to_string()),
        }
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
                        CallBack::State(
                            State::unknown(self.as_profile().get_current_profile().name)
                                .to_string(),
                        )
                    }
                };
                cbs.push(state);

                #[cfg(feature = "connections")]
                let conns = match self.api.get_connections() {
                    Ok(v) => CallBack::ConnectionInit(v),
                    Err(e) => {
                        if !errs.contains(&e.to_string()) {
                            //ensure writing only once
                            log::error!("An error happens in Tick:{e}");
                            errs.push(e.to_string());
                        }
                        CallBack::ConnectionInit(Default::default())
                    }
                };
                #[cfg(feature = "connections")]
                cbs.push(conns);
            } else {
                log::debug!("Backend got:{op:?}");
                let cb = match op {
                    Call::Profile(tabs::profile::BackendOp::Profile(op)) => {
                        self.as_profile().handle_profile_op(op).into()
                    }
                    #[cfg(feature = "template")]
                    Call::Profile(tabs::profile::BackendOp::Template(op)) => {
                        self.as_profile().handle_template_op(op).into()
                    }
                    Call::Service(tabs::service::BackendOp::SwitchMode(mode)) => {
                        // tick will refresh state
                        self.update_state(None, Some(mode))
                            .map(|_| CallBack::ServiceCTL(format!("Mode is switched to {}", mode)))
                            .into()
                    }
                    Call::Service(tabs::service::BackendOp::ServiceCTL(op)) => {
                        self.clash_srv_ctl(op).map(CallBack::ServiceCTL).into()
                    }
                    Call::Service(tabs::service::BackendOp::OpenThis(path)) => {
                        if let Err(e) = crate::utils::ipc::spawn(
                            "sh",
                            vec![
                                "-c",
                                if path.is_dir() {
                                    &self.open_dir_cmd
                                } else {
                                    &self.edit_cmd
                                }
                                .replace("%s", path.to_str().unwrap())
                                .as_str(),
                            ],
                        ) {
                            CallBack::TuiExtend(format!("Failed\n{e}"))
                        } else {
                            CallBack::TuiExtend("Success".to_owned())
                        }
                    }
                    Call::Service(tabs::service::BackendOp::Preview(path)) => {
                        std::fs::read_to_string(path)
                            .map(|string| {
                                CallBack::Preview(string.lines().map(|s| s.to_owned()).collect())
                            })
                            .into()
                    }
                    Call::Service(tabs::service::BackendOp::TuiExtend(extend_op)) => {
                        match extend_op {
                            tabs::service::ExtendOp::ViewClashtuiConfigDir => unreachable!(),
                            tabs::service::ExtendOp::FullLog => logcat(0, 1024)
                                .map(|v| CallBack::TuiExtend(v.join("\n")))
                                .into(),
                            tabs::service::ExtendOp::GenerateInfoList => {
                                let mut infos = vec![
                                    "# CLASHTUI".to_owned(),
                                    format!("version:{}", crate::utils::consts::FULL_VERSION),
                                ];
                                infos.push("# CLASH".to_owned());
                                match self.api.version().and_then(|ver| {
                                    self.api.config_get().map(|cfg| {
                                        let mut cfg = cfg.build();
                                        cfg.insert(2, format!("version:{ver}"));
                                        cfg
                                    })
                                }) {
                                    Ok(info) => {
                                        infos.extend(info);
                                    }
                                    Err(e) => {
                                        infos.push(format!("{e}"));
                                    }
                                };
                                CallBack::TuiExtend(infos.join("\n"))
                            }
                        }
                    }
                    #[cfg(feature = "connections")]
                    Call::Connection(op) => match op {
                        tabs::connection::BackendOp::Terminal(id) => {
                            self.api.terminate_connection(Some(id))
                        }
                        tabs::connection::BackendOp::TerminalAll => {
                            self.api.terminate_connection(None)
                        }
                    }
                    .map(|b| {
                        CallBack::ConnectionCTL(if b {
                            "Success".to_owned()
                        } else {
                            "Failed, log as debug".to_owned()
                        })
                    })
                    .into(),

                    Call::Logs(start, len) => logcat(start, len).map(CallBack::Logs).into(),
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
