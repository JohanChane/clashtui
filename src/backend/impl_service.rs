#[allow(unused_imports)] // currently, only [`SwitchMode`] is impl on macos
use super::ipc::{self, exec};
use super::ClashBackend;
use std::io::Error;

crate::define_enum!(
    #[derive(Clone, Copy)]
    pub enum ServiceOp {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        StartClashService,
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
);

impl ClashBackend {
    #[cfg(target_os = "linux")]
    pub fn clash_srv_ctl(&self, op: ServiceOp) -> Result<String, Error> {
        match op {
            ServiceOp::StartClashService => {
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
            ServiceOp::SetPermission => ipc::exec_with_sbin(
                "setcap",
                vec![
                    "'cap_net_admin,cap_net_bind_service=+ep'",
                    self.cfg.basic.clash_bin_pth.as_str(),
                ],
            ),
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
            ServiceOp::StartClashService => {
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
