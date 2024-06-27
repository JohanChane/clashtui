use super::ClashBackend;
#[allow(unused_imports)] // currently, only [`SwitchMode`] is impl on macos
use crate::utils::ipc::{self, exec};
use crate::utils::ClashSrvOp;
use std::io::Error;

impl ClashBackend {
    #[cfg(target_os = "linux")]
    pub fn clash_srv_ctl(&self, op: ClashSrvOp) -> Result<String, Error> {
        match op {
            ClashSrvOp::StartClashService => {
                let arg = if self.cfg.is_user {
                    vec!["--user", self.cfg.clash_srv_nam.as_str()]
                } else {
                    vec![self.cfg.clash_srv_nam.as_str()]
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
                let arg = if self.cfg.is_user {
                    vec!["--user", self.cfg.clash_srv_nam.as_str()]
                } else {
                    vec![self.cfg.clash_srv_nam.as_str()]
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
                    self.cfg.clash_bin_pth.as_str(),
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
        //let exe_dir = std::env::current_exe()
        //    .unwrap()
        //    .parent()
        //    .unwrap()
        //    .to_path_buf();
        //let nssm_path = exe_dir.join("nssm");
        //let nssm_path_str = nssm_path.to_str().unwrap();
        let nssm_pgm = "nssm";
        use ipc::start_process_as_admin;

        match op {
            ClashSrvOp::StartClashService => {
                start_process_as_admin(
                    nssm_pgm,
                    format!("restart {}", self.cfg.clash_srv_nam).as_str(),
                    true,
                )?;
                exec(nssm_pgm, vec!["status", self.cfg.clash_srv_nam.as_str()])
            }

            ClashSrvOp::StopClashService => {
                start_process_as_admin(
                    nssm_pgm,
                    &format!("stop {}", self.cfg.clash_srv_nam),
                    true,
                )?;
                exec(nssm_pgm, vec!["status", self.cfg.clash_srv_nam.as_str()])
            }

            ClashSrvOp::InstallSrv => {
                start_process_as_admin(
                    nssm_pgm,
                    &format!(
                        "install {} \"{}\" -d \"{}\" -f \"{}\"",
                        self.cfg.clash_srv_nam,
                        self.cfg.clash_bin_pth,
                        self.cfg.clash_cfg_dir,
                        self.cfg.clash_cfg_pth
                    ),
                    true,
                )?;

                exec(nssm_pgm, vec!["status", self.cfg.clash_srv_nam.as_str()])
            }

            ClashSrvOp::UnInstallSrv => ipc::execute_powershell_script_as_admin(
                &format!(
                    "{0} stop {1}; {0} remove {1}",
                    nssm_pgm, self.cfg.clash_srv_nam
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
