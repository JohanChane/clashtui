use super::ClashTuiUtil;
use crate::tui::tabs::ClashSrvOp;
use crate::utils::ipc::{self, exec};
use std::io::Error;

impl ClashTuiUtil {
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
            ClashSrvOp::SetPermission => ipc::exec_with_sbin(
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
}
