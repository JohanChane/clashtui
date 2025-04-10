use super::ClashTuiUtil;
use crate::tui::tabs::ClashSrvOp;
use crate::utils::{
    ipc::{self, exec},
    utils as toolkit,
};

use std::io::Error;

impl ClashTuiUtil {
    pub fn clash_srv_ctl(&self, op: ClashSrvOp) -> Result<String, Error> {
        const RC_SERVCIE_BIN: &str = "/sbin/rc-service";
        if(self.check_file_exist(RC_SERVCIE_BIN).is_ok()){
            return self.clash_srv_ctl_rc_service(op);
        }
        return self.clash_srv_ctl_systemctl(op);
    }
    pub fn clash_srv_ctl_systemctl(&self, op: ClashSrvOp) -> Result<String, Error> {
        match op {
            ClashSrvOp::StartClashService => {
                let mut args = vec!["restart", self.tui_cfg.clash_srv_name.as_str()];
                if self.tui_cfg.is_user {
                    args.push("--user")
                }
                let output1 = exec("systemctl", args)?; // Although the command execution is successful,
                                                        // the operation may not necessarily be successful.
                                                        // So we need to show the command's output to the user.
                args = vec!["status", self.tui_cfg.clash_srv_name.as_str()];
                if self.tui_cfg.is_user {
                    args.push("--user")
                }
                let output2 = exec("systemctl", args)?;

                Ok(format!("# ## restart\n{output1}# ## status\n{output2}"))
            }
            ClashSrvOp::StopClashService => {
                let mut args = vec!["stop", self.tui_cfg.clash_srv_name.as_str()];
                if self.tui_cfg.is_user {
                    args.push("--user")
                }
                let output1 = exec("systemctl", args)?;
                args = vec!["status", self.tui_cfg.clash_srv_name.as_str()];
                if self.tui_cfg.is_user {
                    args.push("--user")
                }
                let output2 = exec("systemctl", args)?;

                Ok(format!("# ## stop\n{output1}# ## status\n{output2}"))
            }
            ClashSrvOp::SetPermission => {
                let pgm = "setcap";
                let args = vec![
                    "cap_net_admin,cap_net_bind_service=+ep",
                    self.tui_cfg.clash_bin_path.as_str(),
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
            ClashSrvOp::CloseConnections => {
                self.clash_api.close_connnections()
            }
            _ => Err(Error::new(
                std::io::ErrorKind::NotFound,
                "No Support Action",
            )),
        }
    }
    pub fn check_file_exist(&self, absolute_path_str: &str) -> Result<bool, Error> {

        let file_path = std::path::PathBuf::from(absolute_path_str);
        if file_path.exists() {
            Ok(true)
        } else {
            Err(Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", file_path.display()),
            ))
        }
    }
    pub fn clash_srv_ctl_rc_service(&self, op: ClashSrvOp) -> Result<String, Error> {
        const SERVICE_CTR_CMD: &str = "rc-service";
        match op {
            ClashSrvOp::StartClashService => {
                let mut args = vec![self.tui_cfg.clash_srv_name.as_str(), "restart"];
                if self.tui_cfg.is_user {
                    args.push("--user")
                }
                let output1 = exec(SERVICE_CTR_CMD, args)?; // Although the command execution is successful,
                                                        // the operation may not necessarily be successful.
                                                        // So we need to show the command's output to the user.
                args = vec![self.tui_cfg.clash_srv_name.as_str(), "status"];
                if self.tui_cfg.is_user {
                    args.push("--user")
                }
                let output2 = exec(SERVICE_CTR_CMD, args)?;

                Ok(format!("# ## restart\n{output1}# ## status\n{output2}"))
            }
            ClashSrvOp::StopClashService => {
                let mut args = vec![self.tui_cfg.clash_srv_name.as_str(), "stop"];
                if self.tui_cfg.is_user {
                    args.push("--user")
                }
                let output1 = exec(SERVICE_CTR_CMD, args)?;
                args = vec![self.tui_cfg.clash_srv_name.as_str(), "status"];
                if self.tui_cfg.is_user {
                    args.push("--user")
                }
                let output2 = exec(SERVICE_CTR_CMD, args)?;

                Ok(format!("# ## stop\n{output1}# ## status\n{output2}"))
            }
            ClashSrvOp::SetPermission => {
                let pgm = "setcap";
                let args = vec![
                    "cap_net_admin,cap_net_bind_service=+ep",
                    self.tui_cfg.clash_bin_path.as_str(),
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
            ClashSrvOp::CloseConnections => {
                self.clash_api.close_connnections()
            }
            _ => Err(Error::new(
                std::io::ErrorKind::NotFound,
                "No Support Action",
            )),
        }
    }
}
