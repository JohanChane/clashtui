use super::ClashTuiUtil;
use crate::tui::tabs::ClashSrvOp;
use crate::utils::ipc::{self, exec};
use std::io::Error;

impl ClashTuiUtil {
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

            ClashSrvOp::UnInstallSrv => ipc::execute_powershell_script_as_admin(
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
