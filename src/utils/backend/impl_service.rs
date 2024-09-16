use super::*;
#[allow(unused_imports)] // currently, only [`SwitchMode`] is impl on macos
use crate::clash::ipc::{self, exec};
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

impl BackEnd {
    pub fn check_update(
        &self,
    ) -> anyhow::Result<Vec<(crate::clash::webapi::github::Response, String)>> {
        Ok(self.api.check_update()?)
    }
    pub fn download_to_file(&self, name: &str, url: &str) -> anyhow::Result<std::path::PathBuf> {
        let path = std::env::current_dir()?.join(name);
        match self.api.get_file(url) {
            Ok(mut rp) => {
                let mut fp = std::fs::File::create(&path)?;
                std::io::copy(&mut rp, &mut fp)
                    .map_err(|e| anyhow::anyhow!("{e}\nincrease timeout might help"))?;
            }
            Err(e) => {
                eprintln!("{e}\ntry to download with `curl/wget`");
                use std::process::{Command, Stdio};
                fn have_this(name: &str) -> bool {
                    Command::new("which")
                        .arg(name)
                        .output()
                        .is_ok_and(|r| r.status.success())
                }
                if have_this("curl") {
                    println!("using curl");
                    Command::new("curl")
                        .args(["-o", &path.to_string_lossy(), "-L", url])
                        .stdin(Stdio::null())
                        .status()?;
                } else if have_this("wget") {
                    println!("using wget");
                    Command::new("wget")
                        .args(["-O", &path.to_string_lossy(), url])
                        .stdin(Stdio::null())
                        .status()?;
                } else {
                    anyhow::bail!("Unable to find curl/wget")
                }
            }
        }
        Ok(path)
    }
    pub fn update_mode(&self, mode: String) -> anyhow::Result<()> {
        let load = format!(r#"{{"mode": "{mode}"}}"#);
        self.api.config_patch(load)?;
        Ok(())
    }
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

impl BackEnd {
    pub fn restart_clash(&self) -> Result<String, String> {
        self.api.restart(None).map_err(|e| e.to_string())
    }
    pub fn update_state(
        &self,
        new_pf: Option<String>,
        new_mode: Option<String>,
    ) -> anyhow::Result<State> {
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
}
