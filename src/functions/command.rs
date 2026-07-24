#[cfg_attr(target_os = "linux", path = "command/linux.rs")]
#[cfg_attr(target_os = "macos", path = "command/macos.rs")]
#[cfg_attr(target_os = "windows", path = "command/windows.rs")]
mod platform;
mod utils;

use crate::config::CONFIG;
use crate::config::{CoreType, ServiceController};
use anyhow::Result;
use std::{path::Path, process::Command};

pub use platform::*;
use utils::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Ops {
    Stop,
    Restart,
    SwitchCore,
    StopAll,
    #[cfg(windows)]
    Install,
    #[cfg(windows)]
    Uninstall,
    #[cfg(windows)]
    ToggleSysProxy,
}

impl Ops {
    fn as_str(&self) -> &str {
        match self {
            Self::Stop => "Stop Service",
            Self::Restart => "Start Service",
            Self::SwitchCore => "Switch Core",
            Self::StopAll => "Stop All Services",
            #[cfg(windows)]
            Self::Install => "Install Srv",
            #[cfg(windows)]
            Self::Uninstall => "Uninstall Srv",
            #[cfg(windows)]
            Self::ToggleSysProxy => "Toggle SysProxy",
        }
    }
    fn all() -> Vec<Self> {
        #[cfg_attr(not(windows), allow(unused_mut))]
        let mut ops = vec![Self::Stop, Self::Restart, Self::SwitchCore, Self::StopAll];
        #[cfg(windows)]
        {
            ops.push(Self::Install);
            ops.push(Self::Uninstall);
            ops.push(Self::ToggleSysProxy);
        }
        ops
    }
}

pub fn test_config(profile_path: Option<&Path>, enable_geodata_mode: bool) -> String {
    let cfg = &CONFIG.cfg_file.mihomo.core;

    let mut cmd = Command::new(&cfg.bin_path);
    cmd.args(["-t", "-d", &cfg.config_dir, "-f"]);
    if let Some(path) = profile_path {
        cmd.arg(path);
    } else {
        cmd.arg(&cfg.config_path);
    }

    if enable_geodata_mode {
        cmd.arg("-m");
    }

    let opt = cmd.output().unwrap();
    stringify_output(opt)
}

pub fn check_config(profile_path: &Path) -> anyhow::Result<()> {
    match CONFIG.core_type() {
        CoreType::Mihomo => {
            let cfg = &CONFIG.cfg_file.mihomo.core;
            let output = Command::new(&cfg.bin_path)
                .args(["-t", "-d", &cfg.config_dir, "-f"])
                .arg(profile_path)
                .output()
                .map_err(|e| anyhow::anyhow!("Failed to run mihomo -t: {e}"))?;
            if output.status.success() {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "mihomo -t failed:\n{}",
                    stringify_output(output)
                ))
            }
        }
        CoreType::Singbox => {
            let cfg = &CONFIG.cfg_file.singbox.core;
            // Strip clashtui metadata before check — sing-box rejects unknown fields
            let check_path = if let Ok(content) = std::fs::read_to_string(profile_path) {
                if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&content) {
                    if value
                        .as_object_mut()
                        .map_or(false, |obj| obj.remove("clashtui").is_some())
                    {
                        let tmp = profile_path.with_file_name(format!(
                            "{}.raw.json",
                            profile_path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("tmp")
                        ));
                        let _ = std::fs::write(
                            &tmp,
                            serde_json::to_string_pretty(&value).unwrap_or_default(),
                        );
                        tmp
                    } else {
                        profile_path.to_path_buf()
                    }
                } else {
                    profile_path.to_path_buf()
                }
            } else {
                profile_path.to_path_buf()
            };
            let output = Command::new(&cfg.bin_path)
                .args(["check", "-D", &cfg.config_dir, "-c"])
                .arg(&check_path)
                .output()
                .map_err(|e| anyhow::anyhow!("Failed to run sing-box check: {e}"))?;
            // Clean up temp file
            if check_path != profile_path {
                let _ = std::fs::remove_file(&check_path);
            }
            if output.status.success() {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "sing-box check failed:\n{}",
                    stringify_output(output)
                ))
            }
        }
    }
}

pub fn is_core_service_running() -> Option<bool> {
    let ct = CONFIG.core_type();
    let scfg = match ct {
        CoreType::Mihomo => &CONFIG.cfg_file.mihomo.core_service,
        CoreType::Singbox => &CONFIG.cfg_file.singbox.core_service,
    };
    let result = exec("sh", vec!["-c", &scfg.is_active]).ok()?;
    Some(result.starts_with("OK")) // Return Code is 0 in fact
}

fn svc_operation(op: Ops, core_type: Option<CoreType>) -> Result<String> {
    let ct = core_type.unwrap_or(CONFIG.core_type());
    let scfg = match ct {
        CoreType::Mihomo => &CONFIG.cfg_file.mihomo.core_service,
        CoreType::Singbox => &CONFIG.cfg_file.singbox.core_service,
    };

    let args = vec![
        "-c",
        match op {
            Ops::Stop => &scfg.stop,
            Ops::Restart => &scfg.restart,
            Ops::SwitchCore => todo!(),
            Ops::StopAll => todo!(),
        },
    ];
    if scfg.need_sudo {
        exec_sudo("sh", args)
    } else {
        exec("sh", args)
    }
}

#[cfg(target_os = "windows")]
fn nssm_svc_operation(op: &str, service_name: &str, ct: CoreType) -> Result<String> {
    match op {
        "start" | "stop" | "restart" | "reload" => {
            let op = if op == "reload" { "restart" } else { op };
            let args = [op, service_name];
            platform::nssm_runas_or_direct(service_name, &args)
        }
        "install" => {
            let bin_path = match ct {
                CoreType::Mihomo => &CONFIG.cfg_file.mihomo.core.bin_path,
                CoreType::Singbox => &CONFIG.cfg_file.singbox.core.bin_path,
            };
            let launch_args = platform::nssm_launch_args(ct);
            let launch_strs: Vec<&str> = launch_args.iter().map(|s| s.as_str()).collect();
            platform::nssm_install(service_name, bin_path, &launch_strs)
        }
        "remove" => platform::nssm_uninstall(service_name),
        _ => Err(anyhow::anyhow!("Unknown nssm operation: {op}")),
    }
}

fn launchd_plist_path(service_name: &str, is_user: bool) -> String {
    if is_user {
        let home = std::env::var("HOME").unwrap_or_default();
        format!("{home}/Library/LaunchAgents/{service_name}.plist")
    } else {
        format!("/Library/LaunchDaemons/{service_name}.plist")
    }
}

fn launchd_operation(op: &str, service_name: &str, is_user: bool) -> Result<String> {
    let plist = launchd_plist_path(service_name, is_user);

    let do_exec = |args: Vec<&str>| -> Result<String> {
        if is_user {
            exec("launchctl", args)
        } else {
            exec_sudo("launchctl", args)
        }
    };

    match op {
        "start" => do_exec(vec!["load", &plist]),
        "stop" => do_exec(vec!["unload", &plist]),
        "restart" | "reload" => {
            // Best-effort unload, then load
            let _ = do_exec(vec!["unload", &plist]);
            do_exec(vec!["load", &plist])
        }
        _ => Err(anyhow::anyhow!("Unknown launchd operation: {op}")),
    }
}

pub fn stop_core_service(core_type: CoreType) -> Result<String> {
    svc_operation("stop", Some(core_type))
}

pub fn start_core_service(core_type: CoreType) -> Result<String> {
    svc_operation("start", Some(core_type))
}

#[cfg(windows)]
pub fn install_core_service(core_type: CoreType) -> Result<String> {
    svc_operation("install", Some(core_type))
}

#[cfg(windows)]
pub fn uninstall_core_service(core_type: CoreType) -> Result<String> {
    svc_operation("remove", Some(core_type))
}

pub fn restart_service() -> Result<String> {
    svc_operation("restart", None)
}

pub fn stop_service() -> Result<String> {
    svc_operation("stop", None)
}

pub fn stop_all_services() -> Result<String> {
    let mut outputs = Vec::new();
    let core_types = [CoreType::Mihomo, CoreType::Singbox];
    for ct in &core_types {
        match stop_core_service(*ct) {
            Ok(out) => outputs.push(out),
            Err(e) => {
                log::warn!("Failed to stop {:?} service: {e}", ct);
            }
        }
    }
    Ok(outputs.join("\n"))
}

pub fn edit(path: &str) -> Result<()> {
    let tpl = CONFIG.cfg_file.extra.edit_cmd.as_deref().unwrap_or("");
    log::debug!("edit: path={path} template={tpl}");
    shell_spawn(tpl, path)
}

pub fn open_dir(path: &str) -> Result<()> {
    let tpl = CONFIG.cfg_file.extra.open_dir_cmd.as_deref().unwrap_or("");
    log::debug!("open_dir: path={path} template={tpl}");
    shell_spawn(tpl, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn test_launchd_plist_path_user() {
        let path = launchd_plist_path("com.example.service", true);
        assert!(path.contains("Library/LaunchAgents"));
        assert!(path.contains("com.example.service.plist"));
        assert!(
            !path.starts_with("/Library/"),
            "user path should use HOME, not system /Library: {path}"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_launchd_plist_path_system() {
        let path = launchd_plist_path("com.example.service", false);
        assert_eq!(path, "/Library/LaunchDaemons/com.example.service.plist");
    }

    #[test]
    fn test_service_controller_args_launchd() {
        let args = ServiceController::Launchd.args("start", "my_service", false);
        assert!(
            args.is_empty(),
            "Launchd args should be empty (handled inline)"
        );
    }

    #[test]
    fn test_service_controller_args_launchd_user() {
        let args = ServiceController::Launchd.args("stop", "my_service", true);
        assert!(args.is_empty(), "Launchd user args should also be empty");
    }

    #[test]
    fn test_service_controller_bin_name_launchd() {
        assert_eq!(ServiceController::Launchd.bin_name(), "launchctl");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_service_controller_default_is_launchd_on_macos() {
        assert_eq!(ServiceController::default(), ServiceController::Launchd);
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn test_service_controller_default_not_macos() {
        // On non-macOS, the default should NOT be Launchd
        assert_ne!(ServiceController::default(), ServiceController::Launchd);
    }
}
