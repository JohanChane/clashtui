use super::*;

use crate::tui::tabs::profile::ProfileOp;
use crate::tui::Call;
use crate::utils::consts::err as consts_err;
use tokio::sync::mpsc::{Receiver, Sender};

impl BackEnd {
    /// Save all in-memory data to file
    fn save(self) -> DataFile {
        let (current_profile, profiles) = self.pm.into_inner();
        DataFile {
            profiles,
            current_profile,
        }
    }
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
    ) -> anyhow::Result<DataFile> {
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
                    Call::Infos => {
                        let mut infos = vec![
                            "# CLASHTUI".to_owned(),
                            format!("version:{}", crate::utils::consts::VERSION),
                        ];
                        match self.api.version().map_err(|e| e.into()).and_then(|ver| {
                            self.api.config_get().map(|cfg| {
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

    fn handle_profile_op(&self, op: ProfileOp) -> CallBack {
        match op {
            ProfileOp::GetALL => {
                let mut composed: Vec<(String, Option<std::time::Duration>)> = self
                    .get_all_profiles()
                    .into_iter()
                    .map(|pf| {
                        (
                            pf.name.clone(),
                            self.load_local_profile(pf).ok().and_then(|lp| lp.atime()),
                        )
                    })
                    .collect();
                composed.sort();
                let (name, atime) = composed.into_iter().collect();
                CallBack::ProfileInit(name, atime)
            }
            ProfileOp::Add(name, url) => {
                self.create_profile(&name, url);
                match self.update_profile(
                    self.get_profile(name)
                        .expect("Cannot find selected profile"),
                    None,
                ) {
                    Ok(v) => CallBack::ProfileCTL(v),
                    Err(e) => CallBack::Error(e.to_string()),
                }
            }
            ProfileOp::Remove(name) => {
                if let Err(e) = self.remove_profile(
                    self.get_profile(name)
                        .expect("Cannot find selected profile"),
                ) {
                    CallBack::Error(e.to_string())
                } else {
                    CallBack::ProfileCTL(vec!["Profile is now removed".to_owned()])
                }
            }
            ProfileOp::Update(name, with_proxy) => {
                match self.update_profile(
                    self.get_profile(name)
                        .expect("Cannot find selected profile"),
                    with_proxy,
                ) {
                    Ok(v) => CallBack::ProfileCTL(v),
                    Err(e) => CallBack::Error(e.to_string()),
                }
            }
            ProfileOp::Select(name) => {
                if let Err(e) = self.select_profile(
                    self.get_profile(name)
                        .expect("Cannot find selected profile"),
                ) {
                    CallBack::Error(e.to_string())
                } else {
                    CallBack::ProfileCTL(vec!["Profile is now loaded".to_owned()])
                }
            }
            ProfileOp::Test(name, geodata_mode) => {
                let pf = self
                    .get_profile(name)
                    .expect("Cannot find selected profile");
                match self.load_local_profile(pf).and_then(|pf| {
                    self.test_profile_config(&pf.path.to_string_lossy(), geodata_mode)
                        .map_err(|e| e.into())
                }) {
                    Ok(v) => CallBack::ProfileCTL(v.lines().map(|s| s.to_owned()).collect()),
                    Err(e) => CallBack::Error(e.to_string()),
                }
            }
            ProfileOp::Preview(name) => {
                let mut lines = Vec::with_capacity(1024);
                let pf = self
                    .get_profile(name)
                    .expect("Cannot find selected profile");
                lines.push(
                    pf.dtype
                        .get_domain()
                        .unwrap_or("Imported local file".to_owned()),
                );
                lines.push(Default::default());
                match self
                    .load_local_profile(pf)
                    .and_then(|pf| match pf.content.as_ref() {
                        Some(content) => {
                            serde_yml::to_string(content)
                                .map_err(|e| e.into())
                                .map(|content| {
                                    lines.extend(content.lines().map(|s| s.to_owned()));
                                })
                        }
                        None => {
                            lines.push("yaml file is empty. Please update it.".to_owned());
                            Ok(())
                        }
                    }) {
                    Ok(()) => CallBack::Preview(lines),
                    Err(e) => CallBack::Error(e.to_string()),
                }
            }
            ProfileOp::Edit(name) => {
                let pf = self
                    .get_profile(name)
                    .expect("Cannot find selected profile");

                match self.load_local_profile(pf).and_then(|pf| {
                    ipc::spawn(
                        "sh",
                        vec![
                            "-c",
                            self.edit_cmd
                                .replace("%s", &pf.path.to_string_lossy())
                                .as_str(),
                        ],
                    )
                    .map_err(|e| e.into())
                }) {
                    Ok(()) => CallBack::Edit,
                    Err(e) => CallBack::Error(e.to_string()),
                }
            }
        }
    }
}
