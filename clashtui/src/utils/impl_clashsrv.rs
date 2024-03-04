use super::ipc::exec;
use super::ClashTuiUtil;
use crate::tui::tabs::ClashSrvOp;
use std::io::Error;

impl ClashTuiUtil {
    pub fn test_profile_config(&self, path: &str, geodata_mode: bool) -> Result<String, Error> {
        let cmd = format!(
            "{} {} -d {} -f {} -t",
            self.tui_cfg.clash_core_path,
            if geodata_mode { "-m" } else { "" },
            self.tui_cfg.clash_cfg_dir,
            path,
        );
        #[cfg(target_os = "windows")]
        return exec("cmd", vec!["/C", cmd.as_str()]);
        #[cfg(target_os = "linux")]
        exec("sh", vec!["-c", cmd.as_str()])
    }

    #[cfg(target_os = "linux")]
    pub fn clash_srv_ctl(&self, op: ClashSrvOp) -> Result<String, Error> {
        match op {
            ClashSrvOp::StartClashService => {
                let mut args = vec!["restart", self.tui_cfg.clash_srv_name.as_str()];
                if self.tui_cfg.is_user {
                    args.push("--user")
                }
                exec("systemctl", args)?;
                exec(
                    "systemctl",
                    vec!["status", self.tui_cfg.clash_srv_name.as_str()],
                )
            }
            ClashSrvOp::StopClashService => {
                let mut args = vec!["stop", self.tui_cfg.clash_srv_name.as_str()];
                if self.tui_cfg.is_user {
                    args.push("--user")
                }
                exec("systemctl", args)?;
                exec(
                    "systemctl",
                    vec!["status", self.tui_cfg.clash_srv_name.as_str()],
                )
            }
            ClashSrvOp::TestClashConfig => {
                self.test_profile_config(self.tui_cfg.clash_cfg_path.as_str(), false)
            }
            ClashSrvOp::UpdateGeoData => self.update_geo(),
            ClashSrvOp::SetPermission => super::ipc::exec_with_sbin(
                "setcap",
                vec![
                    "'cap_net_admin,cap_net_bind_service=+ep'",
                    self.tui_cfg.clash_core_path.as_str(),
                ],
            ),
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
        use super::ipc::start_process_as_admin;

        match op {
            ClashSrvOp::StartClashService => {
                start_process_as_admin(
                    nssm_pgm,
                    format!("restart {}", self.tui_cfg.clash_srv_name).as_str(),
                    true,
                )?;
                exec(
                    nssm_pgm,
                    vec!["status", self.tui_cfg.clash_srv_name.as_str()],
                )
            }

            ClashSrvOp::StopClashService => {
                start_process_as_admin(
                    nssm_pgm,
                    &format!("stop {}", self.tui_cfg.clash_srv_name),
                    true,
                )?;
                exec(
                    nssm_pgm,
                    vec!["status", self.tui_cfg.clash_srv_name.as_str()],
                )
            }

            ClashSrvOp::TestClashConfig => {
                return self.test_profile_config(self.tui_cfg.clash_cfg_path.as_str(), false);
            }

            ClashSrvOp::UpdateGeoData => self.update_geo(),

            ClashSrvOp::InstallSrv => {
                start_process_as_admin(
                    nssm_pgm,
                    &format!(
                        "install {} \"{}\" -d \"{}\" -f \"{}\"",
                        self.tui_cfg.clash_srv_name,
                        self.tui_cfg.clash_core_path,
                        self.tui_cfg.clash_cfg_dir,
                        self.tui_cfg.clash_cfg_path
                    ),
                    true,
                )?;

                exec(
                    nssm_pgm,
                    vec!["status", self.tui_cfg.clash_srv_name.as_str()],
                )
            }

            ClashSrvOp::UnInstallSrv => super::ipc::execute_powershell_script_as_admin(
                &format!(
                    "{0} stop {1}; {0} remove {1}",
                    nssm_pgm, self.tui_cfg.clash_srv_name
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
