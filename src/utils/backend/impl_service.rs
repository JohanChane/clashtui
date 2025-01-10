use super::*;
#[allow(unused_imports)] // currently, only [`SwitchMode`] is impl on macOS
use ipc::exec;
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
    /// check self and clash(current:`mihomo`)
    pub fn check_update(
        &self,
        check_ci: bool,
    ) -> anyhow::Result<Vec<(crate::clash::webapi::github::Response, String)>> {
        use crate::clash::webapi::github::Request;
        let clash_core_version = match self.api.version() {
            Ok(v) => {
                let v: serde_json::Value = serde_json::from_str(&v)?;
                // test mihomo
                let mihomo = v.get("version").and_then(|v| v.as_str());
                // try to get any
                None.or(mihomo).map(|s| s.to_owned())
            }
            Err(_) => None,
        }
        // if None is get, assume there is no clash core installed/running
        .unwrap_or("v0.0.0".to_owned());

        if check_ci {
            return Ok(vec![
                (
                    self.api
                        .get_github_info(&Request::s_clashtui_ci())?
                        .filter_asserts(),
                    crate::consts::VERSION.to_owned(),
                ),
                (
                    self.api
                        .get_github_info(&Request::s_mihomo_ci())?
                        .filter_asserts(),
                    clash_core_version,
                ),
            ]);
        };

        let mut clashtui = self.api.get_github_info(&Request::s_clashtui())?;
        let mut mihomo = self.api.get_github_info(&Request::s_mihomo())?;
        let mut vec = Vec::with_capacity(2);
        if clashtui.is_newer_than(crate::consts::VERSION) {
            clashtui.name = "ClashTUI".to_string();
            vec.push((clashtui.filter_asserts(), crate::consts::VERSION.to_owned()));
        }
        if mihomo.is_newer_than(&clash_core_version) {
            mihomo.name = "Clash Core".to_string();
            vec.push((mihomo.filter_asserts(), clash_core_version))
        }
        Ok(vec)
    }
    pub fn download_to_file(&self, name: &str, url: &str) -> anyhow::Result<std::path::PathBuf> {
        let path = std::env::current_dir()?.join(name);
        match self.api.get_file(url) {
            Ok(mut rp) => {
                let mut fp = std::fs::File::create(&path)?;
                std::io::copy(&mut rp, &mut fp)
                    .map_err(|e| anyhow::anyhow!("{e}, increase timeout might help"))?;
            }
            Err(e) => {
                eprintln!("{e}, try to download with `curl/wget`");
                use std::process::{Command, Stdio};
                fn have_this_and_exec(this: &str, args: &[&str]) -> anyhow::Result<bool> {
                    if Command::new("which")
                        .arg(this)
                        .output()
                        .is_ok_and(|r| r.status.success())
                    {
                        println!("using {this}");
                        if Command::new(this)
                            .args(args)
                            .stdin(Stdio::null())
                            .status()?
                            .success()
                        {
                            Ok(true)
                        } else {
                            Err(anyhow::anyhow!("Failed to download with {this}"))
                        }
                    } else {
                        Ok(false)
                    }
                }
                if !have_this_and_exec("curl", &["-o", &path.to_string_lossy(), "-L", url])? {
                    if !have_this_and_exec("wget", &["-O", &path.to_string_lossy(), url])? {
                        anyhow::bail!("Unable to find curl/wget")
                    }
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
