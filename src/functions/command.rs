#[cfg_attr(target_os = "linux", path = "command/linux.rs")]
mod platform;
mod utils;

use crate::config::CONFIG;
use crate::config::{CoreType, ServiceController};
use anyhow::Result;
use std::{path::Path, process::Command};

pub use platform::*;
use utils::*;

#[cfg(feature = "tui")]
pub async fn resolve_sudo_password(needs_sudo: bool) -> Result<Option<String>> {
    if !needs_sudo {
        return Ok(None);
    }
    if !sudo_needs_password() {
        return Ok(None);
    }
    match crate::tui::prompt_sudo_password().await {
        Some(pw) if pw.is_empty() => Ok(None),
        Some(pw) => Ok(Some(pw)),
        None => Err(anyhow::anyhow!("cancelled")),
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
            let output = Command::new(&cfg.bin_path)
                .args(["check", "-D", &cfg.config_dir, "-c"])
                .arg(profile_path)
                .output()
                .map_err(|e| anyhow::anyhow!("Failed to run sing-box check: {e}"))?;
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

fn svc_operation(op: &str, password: Option<&str>, core_type: Option<CoreType>) -> Result<String> {
    let host = &ServiceController::default();
    let ct = core_type.unwrap_or(CONFIG.core_type());

    let (service_name, is_user) = match ct {
        CoreType::Mihomo => (&CONFIG.cfg_file.mihomo.core_service.service_name, CONFIG.cfg_file.mihomo.core_service.is_user),
        CoreType::Singbox => (&CONFIG.cfg_file.singbox.core_service.service_name, CONFIG.cfg_file.singbox.core_service.is_user),
    };

    let svc_args = host.args(op, service_name, is_user);
    if is_user {
        return exec(host.bin_name(), svc_args);
    }
    let mut argv = vec![host.bin_name()];
    argv.extend(svc_args);

    match password {
        Some(pw) => exec_sudo(argv, pw),
        None => exec("sudo", argv),
    }
}

pub fn stop_core_service(password: Option<&str>, core_type: CoreType) -> Result<String> {
    svc_operation("stop", password, Some(core_type))
}

pub fn restart_core_service(password: Option<&str>, core_type: CoreType) -> Result<String> {
    svc_operation("restart", password, Some(core_type))
}

pub fn reload_core_service(password: Option<&str>, core_type: CoreType) -> Result<String> {
    svc_operation("reload", password, Some(core_type))
}

pub fn restart_service(password: Option<&str>) -> Result<String> {
    svc_operation("restart", password, None)
}

pub fn stop_service(password: Option<&str>) -> Result<String> {
    svc_operation("stop", password, None)
}

pub fn stop_all_services(password: Option<&str>) -> Result<String> {
    let mut outputs = Vec::new();
    let core_types = [CoreType::Mihomo, CoreType::Singbox];
    for ct in &core_types {
        match stop_core_service(password, *ct) {
            Ok(out) => outputs.push(out),
            Err(e) => {
                log::warn!("Failed to stop {:?} service: {e}", ct);
            }
        }
    }
    Ok(outputs.join("\n"))
}

pub fn edit(path: &str) -> Result<()> {
    log::debug!("edit: path={path} cmd={}", CONFIG.cfg_file.extra.edit_cmd);
    spawn(
        "sh",
        vec!["-c", CONFIG.cfg_file.extra.edit_cmd.replace("%s", path).as_str()],
    )
}

pub fn open_dir(path: &str) -> Result<()> {
    log::debug!("open_dir: path={path} cmd={}", CONFIG.cfg_file.extra.open_dir_cmd);
    spawn(
        "sh",
        vec!["-c", CONFIG.cfg_file.extra.open_dir_cmd.replace("%s", path).as_str()],
    )
}

pub fn get_logs() -> String {
    let (service_name, is_user) = match CONFIG.core_type() {
        CoreType::Mihomo => {
            let name = &CONFIG.cfg_file.mihomo.core_service.service_name;
            (if name.is_empty() { "clashtui_mihomo" } else { name.as_str() }, CONFIG.cfg_file.mihomo.core_service.is_user)
        }
        CoreType::Singbox => {
            let name = &CONFIG.cfg_file.singbox.core_service.service_name;
            (if name.is_empty() { "clashtui_singbox" } else { name.as_str() }, CONFIG.cfg_file.singbox.core_service.is_user)
        }
    };

    let mut args = vec!["-u", service_name, "--output=cat", "-n", "200", "--no-pager"];
    if is_user {
        args.insert(0, "--user");
    }

    match std::process::Command::new("journalctl")
        .args(&args)
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if stdout.is_empty() && !stderr.is_empty() {
                stderr
            } else if stdout.is_empty() {
                String::new()
            } else {
                stdout
            }
        }
        Err(e) => format!("Failed to get logs: {e}"),
    }
}
