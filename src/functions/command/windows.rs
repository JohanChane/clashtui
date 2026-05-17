use super::*;
use crate::config::CoreType;
use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::process::Command as StdCommand;

/// Check whether the current process is running with administrator privileges.
fn is_admin() -> bool {
    // `is-root` crate approach: try opening the SAM registry key (admin-only)
    // Fallback: try running a privileged command like `net session`
    StdCommand::new("net")
        .args(["session"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Execute a command with administrator elevation on Windows.
/// Uses PowerShell `Start-Process -Verb RunAs` to trigger UAC,
/// redirects output to temp files, waits, and returns combined output.
fn exec_admin(pgm: &str, args: &[&str]) -> Result<String> {
    let temp = std::env::temp_dir();
    let out_file = temp.join(format!("clashtui_{}_out.txt", fastrand::u32(..)));
    let err_file = temp.join(format!("clashtui_{}_err.txt", fastrand::u32(..)));

    let _ = std::fs::remove_file(&out_file);
    let _ = std::fs::remove_file(&err_file);

    let arg_string = args
        .iter()
        .map(|a| format!("'{}'", a.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(",");

    let ps_cmd = format!(
        "Start-Process -FilePath '{}' -ArgumentList {} -Verb RunAs -Wait -WindowStyle Hidden -RedirectStandardOutput '{}' -RedirectStandardError '{}'",
        pgm.replace('\'', "''"),
        arg_string,
        out_file.display(),
        err_file.display(),
    );

    log::debug!("IPC (admin): {}", ps_cmd);

    let ps_output = StdCommand::new("powershell")
        .args(["-NoProfile", "-Command", &ps_cmd])
        .output()
        .map_err(|e| anyhow!("Failed to launch PowerShell for elevation: {e}"))?;

    if !ps_output.status.success() {
        let ps_err = String::from_utf8_lossy(&ps_output.stderr);
        if ps_err.contains("canceled") || ps_err.contains("denied") {
            return Err(anyhow!("Administrator elevation was denied or cancelled"));
        }
        // Continue anyway, maybe the process launched but exited non-zero
    }

    let stdout = std::fs::read_to_string(&out_file).unwrap_or_default();
    let stderr = std::fs::read_to_string(&err_file).unwrap_or_default();

    let _ = std::fs::remove_file(&out_file);
    let _ = std::fs::remove_file(&err_file);

    let combined = if stderr.is_empty() {
        stdout
    } else if stdout.is_empty() {
        stderr
    } else {
        format!("{}\n{}", stdout, stderr)
    };

    if combined.trim().is_empty() {
        return Ok(String::new());
    }

    Ok(combined)
}

/// Call nssm with elevation when admin rights are needed.
fn exec_nssm(args: &[&str]) -> Result<String> {
    if is_admin() {
        exec("nssm", args.to_vec())
    } else {
        exec_admin("nssm", args)
    }
}

// ── File permission stubs ──────────────────────────────────────

pub fn correct_cap_for_tun() -> Result<String> {
    Ok("No setcap on Windows".into())
}

pub fn find_files_not_group_writable(_dir: &Path) -> Vec<PathBuf> {
    Vec::new()
}

pub fn find_files_not_in_group(_dir: &Path, _group_name: &str) -> Vec<PathBuf> {
    Vec::new()
}

pub fn get_dir_group_name(_dir: &Path) -> Option<String> {
    Some(String::new())
}

pub fn check_file_permissions(_dir: &Path) -> bool {
    true
}

pub fn repair_file_permissions(_dir: &Path, _group_name: &str) -> Result<String> {
    Ok("Permissions OK on Windows".into())
}

// ── Service operations via nssm ────────────────────────────────

/// Build the service binary launch arguments based on core type.
pub fn service_launch_args(ct: CoreType) -> Vec<String> {
    let cfg = &crate::config::CONFIG.cfg_file;
    match ct {
        CoreType::Mihomo => vec![
            "-d".to_owned(),
            cfg.mihomo.core.config_dir.clone(),
        ],
        CoreType::Singbox => vec![
            "-D".to_owned(),
            cfg.singbox.core.config_dir.clone(),
            "-c".to_owned(),
            cfg.singbox.core.config_path.clone(),
            "run".to_owned(),
        ],
    }
}

pub fn service_bin_path(ct: CoreType) -> String {
    let cfg = &crate::config::CONFIG.cfg_file;
    match ct {
        CoreType::Mihomo => cfg.mihomo.core.bin_path.clone(),
        CoreType::Singbox => cfg.singbox.core.bin_path.clone(),
    }
}

pub fn service_name_for(ct: CoreType) -> String {
    let cfg = &crate::config::CONFIG.cfg_file;
    let name = match ct {
        CoreType::Mihomo => &cfg.mihomo.core_service.service_name,
        CoreType::Singbox => &cfg.singbox.core_service.service_name,
    };
    if name.is_empty() {
        match ct {
            CoreType::Mihomo => "clashtui_mihomo",
            CoreType::Singbox => "clashtui_singbox",
        }
        .to_owned()
    } else {
        name.clone()
    }
}

/// Start or stop a Windows service via nssm.
pub fn windows_service_operation(op: &str, service_name: &str) -> Result<String> {
    let nssm_op = match op {
        "start" => "start",
        "stop" => "stop",
        "restart" | "reload" => "restart",
        _ => return Err(anyhow!("Unknown Windows service operation: {op}")),
    };
    exec_nssm(&[nssm_op, service_name])
}

/// Install a Windows service via nssm.
pub fn windows_service_install(
    _ct: CoreType,
    bin_path: &str,
    service_name: &str,
    launch_args: &[String],
) -> Result<String> {
    let mut args: Vec<&str> = vec!["install", service_name, bin_path];
    let string_args: Vec<String> = launch_args.iter().map(|a| a.as_str().to_owned()).collect();
    for arg in &string_args {
        args.push(arg.as_str());
    }
    exec_nssm(&args)
}

/// Uninstall a Windows service via nssm.
pub fn windows_service_uninstall(service_name: &str) -> Result<String> {
    exec_nssm(&["remove", service_name, "confirm"])
}

/// Query Windows service status via nssm.
pub fn windows_service_status(service_name: &str) -> String {
    let result = if is_admin() {
        StdCommand::new("nssm")
            .args(["status", service_name])
            .output()
            .map(|o| (o.status.success(), String::from_utf8_lossy(&o.stdout).to_string()))
            .map_err(|e| anyhow!(e))
    } else {
        exec_admin("nssm", &["status", service_name])
            .map(|s| (true, s))
    };

    match result {
        Ok((_, stdout)) => {
            if stdout.contains("SERVICE_RUNNING") {
                "active".to_owned()
            } else if stdout.contains("SERVICE_STOPPED") {
                "inactive".to_owned()
            } else if stdout.contains("SERVICE_PAUSED") {
                "inactive".to_owned()
            } else {
                "uninstalled".to_owned()
            }
        }
        Err(_) => "?".to_owned(),
    }
}

// ── System proxy toggle via registry ──────────────────────────

/// Read the Windows system proxy state from registry.
pub fn get_system_proxy_state() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    let internet_settings = match hkcu.open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Internet Settings",
        KEY_READ,
    ) {
        Ok(k) => k,
        Err(_) => return false,
    };

    let proxy_enable: u32 = internet_settings.get_value("ProxyEnable").unwrap_or(0);
    proxy_enable != 0
}

/// Enable or disable the Windows system proxy.
/// `mixed_port` is the core's mixed inbound port (e.g., 7890).
pub fn toggle_system_proxy(enable: bool, mixed_port: u16) -> Result<String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let internet_settings = hkcu.open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Internet Settings",
        KEY_WRITE,
    )?;

    if enable {
        internet_settings.set_value("ProxyEnable", &1u32)?;
        internet_settings
            .set_value("ProxyServer", &format!("127.0.0.1:{mixed_port}"))?;
        internet_settings.set_value("ProxyOverride", &"<-loopback>")?;
        broadcast_settings_change();
        Ok("System proxy enabled".into())
    } else {
        internet_settings.set_value("ProxyEnable", &0u32)?;
        broadcast_settings_change();
        Ok("System proxy disabled".into())
    }
}

/// Broadcast `WM_SETTINGCHANGE` to notify system of proxy changes.
fn broadcast_settings_change() {
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        SendMessageTimeoutW, HWND_BROADCAST, SMTO_NORMAL, WM_SETTINGCHANGE,
    };

    let env_str = "Environment";
    let env_utf16: Vec<u16> = env_str.encode_utf16().chain(std::iter::once(0)).collect();
    let lparam = LPARAM(env_utf16.as_ptr() as isize);

    let result = unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            lparam,
            SMTO_NORMAL,
            5000,
            None,
        )
    };
    if result.0 == 0 {
        log::warn!("WM_SETTINGCHANGE broadcast may have failed");
    }
}

/// Get the mixed inbound port from the current core's REST API.
pub fn get_mixed_port() -> Option<u16> {
    let config = &crate::config::CONFIG;
    let controller = config.controller_for_core();
    let secret = config.secret_for_core();

    let url = format!("{controller}/configs");
    let mut req = minreq::get(&url).with_timeout(5);
    if let Some(s) = &secret {
        req = req.with_header("Authorization", format!("Bearer {s}"));
    }

    let response = match req.send() {
        Ok(r) => r,
        Err(e) => {
            log::warn!("Failed to get configs for mixed port: {e}");
            return None;
        }
    };

    let body = response.as_str().ok()?;
    let json: serde_json::Value = serde_json::from_str(body).ok()?;
    json.get("mixed-port")
        .or_else(|| json.get("port"))
        .and_then(|v| v.as_u64())
        .map(|p| p as u16)
}

// ── Helpers ─────────────────────────────────────────────────────

pub(super) fn stringify_output(output: std::process::Output) -> String {
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);

    let result_str = format!(
        r#"{}
        Stdout:
        {}

        Stderr:
        {}
        "#,
        if output.status.success() {
            "OK".to_owned()
        } else {
            format!("Error({})", output.status)
        },
        stdout_str,
        stderr_str
    );

    result_str
}
