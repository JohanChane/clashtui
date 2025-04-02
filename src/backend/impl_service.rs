use super::{ipc, BackEnd};

#[allow(unused_imports)] // currently, only [`SwitchMode`] is impl on macOS
use ipc::exec;
use std::io::Error;

mod state;

pub(super) use state::State;

#[cfg_attr(
    feature = "tui",
    derive(strum::Display, strum::EnumCount, strum::VariantArray)
)]
#[derive(Clone, Copy, Debug)]
pub enum ServiceOp {
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    RestartClashService,
    RestartClashCore,
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    StopClashService,
    #[cfg(target_os = "linux")]
    SetPermission,
    #[cfg(target_os = "windows")]
    SwitchSysProxy,
    #[cfg(target_os = "windows")]
    EnableLoopback,
    #[cfg(target_os = "windows")]
    InstallSrv,
    #[cfg(target_os = "windows")]
    UnInstallSrv,
}

impl BackEnd {
    pub fn update_mode(&self, mode: crate::clash::webapi::Mode) -> anyhow::Result<()> {
        let load = format!(r#"{{"mode": "{mode}"}}"#);
        self.api.config_patch(load)?;
        Ok(())
    }
    #[cfg(target_os = "linux")]
    pub fn clash_srv_ctl(&self, op: ServiceOp) -> Result<String, Error> {
        match op {
            ServiceOp::RestartClashService => {
                let arg = if self.cfg.service.is_user {
                    vec!["--user", self.cfg.service.clash_srv_nam.as_str()]
                } else {
                    vec![self.cfg.service.clash_srv_nam.as_str()]
                };
                {
                    let mut args = vec!["restart"];
                    args.extend(arg.iter());
                    exec("systemctl", args)?;
                }
                {
                    let mut args = vec!["status"];
                    args.extend(arg.iter());
                    exec("systemctl", args)
                }
            }
            ServiceOp::RestartClashCore => self.restart_clash(),
            ServiceOp::StopClashService => {
                let arg = if self.cfg.service.is_user {
                    vec!["--user", self.cfg.service.clash_srv_nam.as_str()]
                } else {
                    vec![self.cfg.service.clash_srv_nam.as_str()]
                };
                {
                    let mut args = vec!["stop"];
                    args.extend(arg.iter());
                    exec("systemctl", args)?;
                }
                {
                    let mut args = vec!["status"];
                    args.extend(arg.iter());
                    exec("systemctl", args)
                }
            }
            ServiceOp::SetPermission => {
                exec("chmod", vec!["+x", self.cfg.basic.clash_bin_pth.as_str()])?;
                ipc::exec_with_sbin(
                    "setcap",
                    vec![
                        "'cap_net_admin,cap_net_bind_service=+ep'",
                        self.cfg.basic.clash_bin_pth.as_str(),
                    ],
                )
            }
            #[allow(unreachable_patterns)]
            _ => Err(Error::new(
                std::io::ErrorKind::NotFound,
                "No Support Action",
            )),
        }
    }
    #[cfg(target_os = "macos")]
    pub fn clash_srv_ctl(&self, op: ServiceOp) -> Result<String, Error> {
        match op {
            ServiceOp::RestartClashCore => self.restart_clash(),
            #[allow(unreachable_patterns)]
            _ => Err(Error::new(
                std::io::ErrorKind::NotFound,
                "No Support Action",
            )),
        }
    }
    #[cfg(target_os = "windows")]
    pub fn clash_srv_ctl(&self, op: ServiceOp) -> Result<String, Error> {
        const NSSM_PGM: &str = "nssm";

        use ipc::start_process_as_admin;

        match op {
            ServiceOp::RestartClashService => {
                start_process_as_admin(
                    NSSM_PGM,
                    format!("restart {}", self.cfg.service.clash_srv_nam).as_str(),
                    true,
                )?;
                exec(
                    NSSM_PGM,
                    vec!["status", self.cfg.service.clash_srv_nam.as_str()],
                )
            }

            ServiceOp::RestartClashCore => self.restart_clash(),

            ServiceOp::StopClashService => {
                start_process_as_admin(
                    NSSM_PGM,
                    &format!("stop {}", self.cfg.service.clash_srv_nam),
                    true,
                )?;
                exec(
                    NSSM_PGM,
                    vec!["status", self.cfg.service.clash_srv_nam.as_str()],
                )
            }

            ServiceOp::InstallSrv => {
                start_process_as_admin(
                    NSSM_PGM,
                    &format!(
                        "install {} \"{}\" -d \"{}\" -f \"{}\"",
                        self.cfg.service.clash_srv_nam,
                        self.cfg.basic.clash_bin_pth,
                        self.cfg.basic.clash_cfg_dir,
                        self.cfg.basic.clash_cfg_pth
                    ),
                    true,
                )?;

                exec(
                    NSSM_PGM,
                    vec!["status", self.cfg.service.clash_srv_nam.as_str()],
                )
            }

            ServiceOp::UnInstallSrv => ipc::execute_powershell_script_as_admin(
                &format!(
                    "{0} stop {1}; {0} remove {1}",
                    NSSM_PGM, self.cfg.service.clash_srv_nam
                ),
                true,
            ),

            ServiceOp::EnableLoopback => {
                let exe_dir = std::env::current_exe()?
                    .parent()
                    .expect("Exec at / ?")
                    .to_path_buf();
                start_process_as_admin(exe_dir.join("EnableLoopback").to_str().unwrap(), "", false)
            }

            ServiceOp::SwitchSysProxy => {
                let current = self.is_system_proxy_enabled()?;
                if current {
                    self.disable_system_proxy()
                } else {
                    self.enable_system_proxy()
                }
            }
            #[allow(unreachable_patterns)]
            _ => Err(Error::new(
                std::io::ErrorKind::NotFound,
                "No Support Action",
            )),
        }
    }
    #[cfg(windows)]
    pub fn enable_system_proxy(&self) -> std::io::Result<String> {
        ipc::enable_system_proxy(&self.api.proxy_addr)
    }
    #[cfg(windows)]
    pub fn disable_system_proxy(&self) -> std::io::Result<String> {
        ipc::disable_system_proxy()
    }
    #[cfg(windows)]
    pub fn is_system_proxy_enabled(&self) -> std::io::Result<bool> {
        ipc::is_system_proxy_enabled()
    }
}

impl BackEnd {
    pub fn restart_clash(&self) -> std::io::Result<String> {
        self.api.restart(None).map_err(Error::other)
    }
    pub fn update_state(
        &self,
        new_pf: Option<String>,
        new_mode: Option<crate::clash::webapi::Mode>,
    ) -> anyhow::Result<State> {
        use crate::clash::webapi::ClashConfig;
        if let Some(mode) = new_mode {
            self.update_mode(mode)?;
        }
        if let Some(pf) = new_pf.as_ref() {
            if let Some(pf) = self.get_profile(pf) {
                self.select_profile(pf)?;
            } else {
                anyhow::bail!("Not a recorded profile");
            };
        }
        #[cfg(target_os = "windows")]
        let sysp = self.is_system_proxy_enabled().map_or_else(
            |v| {
                log::error!("{}", v);
                None
            },
            Some,
        );
        let ClashConfig { mode, tun, .. } = self.api.config_get()?;
        Ok(State {
            profile: new_pf.unwrap_or(self.get_current_profile().name),
            mode: Some(mode),
            tun: if tun.enable { Some(tun.stack) } else { None },
            #[cfg(target_os = "windows")]
            sysproxy: sysp,
        })
    }
    pub fn get_clash_version(&self) -> Result<String, String> {
        let v = self.api.version().map_err(|e| e.to_string())?;
        let map: serde_json::Value = serde_json::from_str(&v).unwrap();
        Ok(map
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("v0.0.0")
            .to_owned())
    }
}
