use super::ClashTuiUtil;
use crate::tui::tabs::ClashSrvOp;
use crate::utils::{
    ipc::{self, exec},
    utils as toolkit,
};

use std::io::Error;

impl ClashTuiUtil {
    pub fn clash_srv_ctl(&self, op: ClashSrvOp) -> Result<String, Error> {
        match op {
            ClashSrvOp::StartClashService => {
                let mut args = vec!["restart", self.tui_cfg.clash_srv_name.as_str()];
                if self.tui_cfg.clash_srv_is_user {
                    args.push("--user")
                }
                let output1 = exec("systemctl", args)?; // Although the command execution is successful,
                                                        // the operation may not necessarily be successful.
                                                        // So we need to show the command's output to the user.
                args = vec!["status", self.tui_cfg.clash_srv_name.as_str()];
                if self.tui_cfg.clash_srv_is_user {
                    args.push("--user")
                }
                let output2 = exec("systemctl", args)?;

                Ok(format!("# ## restart\n{output1}# ## status\n{output2}"))
            }
            ClashSrvOp::StopClashService => {
                let mut args = vec!["stop", self.tui_cfg.clash_srv_name.as_str()];
                if self.tui_cfg.clash_srv_is_user {
                    args.push("--user")
                }
                let output1 = exec("systemctl", args)?;
                args = vec!["status", self.tui_cfg.clash_srv_name.as_str()];
                if self.tui_cfg.clash_srv_is_user {
                    args.push("--user")
                }
                let output2 = exec("systemctl", args)?;

                Ok(format!("# ## stop\n{output1}# ## status\n{output2}"))
            }
            ClashSrvOp::SetPermission => {
                let pgm = "setcap";
                let args = vec![
                    "cap_net_admin,cap_net_bind_service=+ep",
                    self.tui_cfg.clash_core_path.as_str(),
                ];
                if toolkit::is_run_as_root() {
                    ipc::exec_with_sbin(pgm, args)
                } else {
                    let mut cmd = vec![pgm];
                    cmd.extend(args);
                    // `setcap` doesn't trigger the polkit agent.
                    ipc::exec_with_sbin("pkexec", cmd)
                }
            }
            _ => Err(Error::new(
                std::io::ErrorKind::NotFound,
                "No Support Action",
            )),
        }
    }
}
