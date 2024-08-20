#[allow(unused_imports)] // currently, only [`SwitchMode`] is impl on macos
use super::ipc::{self, exec};
use super::ClashBackend;
use std::io::Error;

impl ClashBackend {
    #[cfg(target_os = "linux")]
    pub fn clash_srv_ctl(&self, op: ClashSrvOp) -> Result<String, Error> {
        match op {
            ClashSrvOp::StartClashService => {
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
            ClashSrvOp::StopClashService => {
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
            ClashSrvOp::SetPermission => ipc::exec_with_sbin(
                "setcap",
                vec![
                    "'cap_net_admin,cap_net_bind_service=+ep'",
                    self.cfg.basic.clash_bin_pth.as_str(),
                ],
            ),
            _ => Err(Error::new(
                std::io::ErrorKind::NotFound,
                "No Support Action",
            )),
        }
    }
    #[cfg(target_os = "macos")]
    pub fn clash_srv_ctl(&self, op: ClashSrvOp) -> Result<String, Error> {
        match op {
            _ => Err(Error::new(
                std::io::ErrorKind::NotFound,
                "No Support Action",
            )),
        }
    }
    #[cfg(target_os = "windows")]
    pub fn clash_srv_ctl(&self, op: ClashSrvOp) -> Result<String, Error> {
        let nssm_pgm = "nssm";
        use ipc::start_process_as_admin;

        match op {
            ClashSrvOp::StartClashService => {
                start_process_as_admin(
                    nssm_pgm,
                    format!("restart {}", self.cfg.service.clash_srv_nam).as_str(),
                    true,
                )?;
                exec(
                    nssm_pgm,
                    vec!["status", self.cfg.service.clash_srv_nam.as_str()],
                )
            }

            ClashSrvOp::StopClashService => {
                start_process_as_admin(
                    nssm_pgm,
                    &format!("stop {}", self.cfg.service.clash_srv_nam),
                    true,
                )?;
                exec(
                    nssm_pgm,
                    vec!["status", self.cfg.service.clash_srv_nam.as_str()],
                )
            }

            ClashSrvOp::InstallSrv => {
                start_process_as_admin(
                    nssm_pgm,
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
                    nssm_pgm,
                    vec!["status", self.cfg.service.clash_srv_nam.as_str()],
                )
            }

            ClashSrvOp::UnInstallSrv => ipc::execute_powershell_script_as_admin(
                &format!(
                    "{0} stop {1}; {0} remove {1}",
                    nssm_pgm, self.cfg.service.clash_srv_nam
                ),
                true,
            ),

            ClashSrvOp::EnableLoopback => {
                let exe_dir = std::env::current_exe()?
                    .parent()
                    .expect("Exec at / ?")
                    .to_path_buf();
                start_process_as_admin(exe_dir.join("EnableLoopback").to_str().unwrap(), "", false)
            }
            _ => Err(Error::new(
                std::io::ErrorKind::NotFound,
                "No Support Action",
            )),
        }
    }
}

#[cfg(target_os = "linux")]
crate::define_enum!(
    pub ClashSrvOp,
    [
        StartClashService,
        StopClashService,
        SetPermission,
        SwitchMode
    ]
);
#[cfg(target_os = "macos")]
crate::define_enum!(
    pub ClashSrvOp,
    [
        SwitchMode
    ]
);
#[cfg(target_os = "windows")]
crate::define_enum!(
    pub ClashSrvOp,
    [
        StartClashService,
        StopClashService,
        SwitchSysProxy,
        EnableLoopback,
        InstallSrv,
        UnInstallSrv,
        SwitchMode
    ]
);
