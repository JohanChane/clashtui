use crate::utils::config::ServiceController;

use super::{BackEnd, Mode, State, clash::webapi::ClashConfig, ipc};

#[allow(unused_imports)] // currently, only [`SwitchMode`] is impl on macOS
use ipc::exec;
use std::io::Error;

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
    pub fn restart_clash(&self) -> std::io::Result<String> {
        self.api.restart(None).map_err(Error::other)
    }
    pub fn update_state(
        &self,
        new_pf: Option<String>,
        new_mode: Option<Mode>,
    ) -> anyhow::Result<State> {
        if let Some(mode) = new_mode {
            self.update_mode(mode)?;
        }
        if let Some(pf) = new_pf.as_ref() {
            if let Some(pf) = self.as_profile().get_profile(pf) {
                self.as_profile().select_profile(pf)?;
            } else {
                anyhow::bail!("Not a recorded profile");
            };
        }
        #[cfg(target_os = "windows")]
        let sysproxy = self.is_system_proxy_enabled().map_or_else(
            |v| {
                log::error!("{}", v);
                None
            },
            Some,
        );
        let ClashConfig { mode, tun, .. } = self.api.config_get()?;
        Ok(State {
            profile: new_pf.unwrap_or(self.as_profile().get_current_profile().name),
            mode: Some(mode),
            tun: if tun.enable { Some(tun.stack) } else { None },
            #[cfg(target_os = "windows")]
            sysproxy,
        })
    }
    pub fn get_clash_version(&self) -> anyhow::Result<String> {
        let v = self.api.version()?;
        let map: serde_json::Value = serde_json::from_str(&v).unwrap();
        Ok(map
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("v0.0.0")
            .to_owned())
    }
    pub fn update_mode(&self, mode: Mode) -> anyhow::Result<()> {
        let load = format!(r#"{{"mode": "{mode}"}}"#);
        self.api.config_patch(load)?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub fn clash_srv_ctl(&self, op: ServiceOp) -> Result<String, Error> {
        let service_controller = &self.cfg.hack.service_controller;
        let is_user = self.cfg.service.is_user;
        let service_name = &self.cfg.service.clash_service_name;
        match op {
            ServiceOp::RestartClashService => service_controller.restart(service_name, is_user),
            ServiceOp::RestartClashCore => self.restart_clash(),
            ServiceOp::StopClashService => service_controller.stop(service_name, is_user),
            ServiceOp::SetPermission => ipc::set_clash_permission(&self.cfg.basic.clash_bin_path),
            
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
                    format!("restart {}", self.cfg.service.clash_service_name).as_str(),
                    true,
                )?;
                exec(
                    NSSM_PGM,
                    vec!["status", self.cfg.service.clash_service_name.as_str()],
                )
            }

            ServiceOp::RestartClashCore => self.restart_clash(),

            ServiceOp::StopClashService => {
                start_process_as_admin(
                    NSSM_PGM,
                    &format!("stop {}", self.cfg.service.clash_service_name),
                    true,
                )?;
                exec(
                    NSSM_PGM,
                    vec!["status", self.cfg.service.clash_service_name.as_str()],
                )
            }

            ServiceOp::InstallSrv => {
                start_process_as_admin(
                    NSSM_PGM,
                    &format!(
                        "install {} \"{}\" -d \"{}\" -f \"{}\"",
                        self.cfg.service.clash_service_name,
                        self.cfg.basic.clash_bin_path,
                        self.cfg.basic.clash_config_dir,
                        self.cfg.basic.clash_config_path
                    ),
                    true,
                )?;

                exec(
                    NSSM_PGM,
                    vec!["status", self.cfg.service.clash_service_name.as_str()],
                )
            }

            ServiceOp::UnInstallSrv => ipc::execute_powershell_script_as_admin(
                &format!(
                    "{0} stop {1}; {0} remove {1}",
                    NSSM_PGM, self.cfg.service.clash_service_name
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

trait ServiceExt {
    fn status(&self, service_name: &str, is_user: bool) -> std::io::Result<String>;
    fn restart(&self, service_name: &str, is_user: bool) -> std::io::Result<String>;
    fn stop(&self, service_name: &str, is_user: bool) -> std::io::Result<String>;
}
impl ServiceExt for ServiceController {
    fn status(&self, service_name: &str, is_user: bool) -> std::io::Result<String> {
        let args = self.apply_args("status", service_name, is_user);
        exec(self.bin_name(), args)
    }

    fn restart(&self, service_name: &str, is_user: bool) -> std::io::Result<String> {
        let args = self.apply_args("restart", service_name, is_user);
        exec(self.bin_name(), args)?;
        self.status(service_name, is_user)
    }

    fn stop(&self, service_name: &str, is_user: bool) -> std::io::Result<String> {
        let args = self.apply_args("stop", service_name, is_user);
        exec(self.bin_name(), args)?;
        self.status(service_name, is_user)
    }
}
