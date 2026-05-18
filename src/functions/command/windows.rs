use super::*;
use std::path::PathBuf;

// ============================================================================
// Platform stubs — file permissions / TUN capability (no-ops on Windows)
// ============================================================================

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
    None
}

pub fn check_file_permissions(_dir: &Path) -> bool {
    true
}

pub fn repair_file_permissions(_dir: &Path, _group_name: &str) -> Result<String> {
    Ok("Permissions OK on Windows".into())
}

pub(super) fn stringify_output(output: std::process::Output) -> String {
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);

    format!(
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
    )
}

// ============================================================================
// nssm CLI service operations
// ============================================================================

use crate::config::CoreType;
use crate::config::CONFIG;

fn nssm_bin() -> &'static str {
    "nssm"
}

fn nssm_cmd(args: &[&str]) -> Result<std::process::Output> {
    std::process::Command::new(nssm_bin())
        .args(args)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run nssm: {e}"))
}

/// Install a Windows service via nssm (auto-elevates via UAC if needed).
/// `service_name`: service display name
/// `bin_path`: full path to the .exe
/// `launch_args`: arguments passed to the binary at service start
pub fn nssm_install(service_name: &str, bin_path: &str, launch_args: &[&str]) -> Result<String> {
    let mut args = vec!["install", service_name, bin_path];
    args.extend(launch_args);
    nssm_runas_or_direct(service_name, &args)
}

/// Uninstall a Windows service via nssm (auto-elevates via UAC if needed).
/// Stops the service first, then removes it.
pub fn nssm_uninstall(service_name: &str) -> Result<String> {
    // Best-effort stop before remove — ignore errors (service may already be stopped)
    let _ = nssm_runas_or_direct(service_name, &["stop", service_name]);
    let args = ["remove", service_name, "confirm"];
    nssm_runas_or_direct(service_name, &args)
}

/// Query nssm service status.
/// Returns: "active" | "inactive" | "uninstalled" | "?"
pub fn nssm_status(service_name: &str) -> String {
    let output = match nssm_cmd(&["status", service_name]) {
        Ok(o) => o,
        Err(_) => return "uninstalled".to_owned(),
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stdout = stdout.trim().to_uppercase();
    if stdout.contains("SERVICE_RUNNING") {
        "active".to_owned()
    } else if stdout.contains("SERVICE_STOPPED") || stdout.contains("SERVICE_PAUSED") {
        "inactive".to_owned()
    } else {
        "uninstalled".to_owned()
    }
}

/// Build nssm launch args for the current core type.
/// mihomo: `-d <config_dir>`
/// sing-box: `-D <config_dir> -c <config_path> run`
pub fn nssm_launch_args(ct: CoreType) -> Vec<String> {
    match ct {
        CoreType::Mihomo => {
            let cfg = &CONFIG.cfg_file.mihomo.core;
            vec!["-d".to_owned(), cfg.config_dir.clone()]
        }
        CoreType::Singbox => {
            let cfg = &CONFIG.cfg_file.singbox.core;
            vec![
                "-D".to_owned(),
                cfg.config_dir.clone(),
                "-c".to_owned(),
                cfg.config_path.clone(),
                "run".to_owned(),
            ]
        }
    }
}

// ============================================================================
// Elevation
// ============================================================================

pub fn is_elevated() -> bool {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    let mut token = HANDLE::default();
    unsafe {
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation: TOKEN_ELEVATION = std::mem::zeroed();
        let mut size: u32 = std::mem::size_of::<TOKEN_ELEVATION>() as u32;
        let result = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut std::ffi::c_void),
            size,
            &mut size,
        );
        let _ = CloseHandle(token);
        result.is_ok() && elevation.TokenIsElevated != 0
    }
}

/// Run a command via PowerShell's `Start-Process -Verb RunAs` (UAC elevation).
/// Distinguishes UAC cancellation (exit code 1223) from command failure.
fn runas(cmd: &str, args: &[&str]) -> Result<()> {
    let arg_list = args
        .iter()
        .map(|a| a.replace('\\', "/"))
        .collect::<Vec<_>>()
        .join(" ");
    // PowerShell: catch Start-Process errors (e.g. UAC cancelled) and
    // exit with 1223 (ERROR_CANCELLED).  On success, exit with nssm's
    // exit code so we can tell whether the command itself failed.
    let ps = format!(
        "\
try {{
    $p = Start-Process -FilePath '{cmd}' -ArgumentList '{arg_list}' \
         -Verb RunAs -Wait -PassThru -ErrorAction Stop
    exit $p.ExitCode
}} catch {{
    exit 1223
}}"
    );
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to launch PowerShell: {e}"))?;

    let code = output.status.code().unwrap_or(-1);
    if code == 0 {
        return Ok(());
    }
    if code == 1223 {
        return Err(anyhow::anyhow!(
            "Administrator privileges are required to manage services.\n\
             UAC prompt was cancelled or could not be displayed.\n\
             Please accept the UAC prompt to continue."
        ));
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(anyhow::anyhow!(
        "'{cmd}' failed with exit code {code}.\n\
         Command: {cmd} {arg_list}\n\
         {stderr}",
        stderr = stderr.trim()
    ))
}

/// Run an nssm command, auto-elevating via UAC if needed.
/// For start/restart: verifies the service is actually running afterwards.
pub fn nssm_runas_or_direct(service_name: &str, nssm_args: &[&str]) -> Result<String> {
    let op = nssm_args.first().copied().unwrap_or("");
    if is_elevated() {
        let output = std::process::Command::new(nssm_bin())
            .args(nssm_args)
            .output()?;
        if output.status.success() {
            return Ok(stringify_output(output));
        }
        return Err(anyhow::anyhow!("{}", stringify_output(output)));
    }
    runas(nssm_bin(), nssm_args)?;
    // After the elevated operation, query status (no elevation needed)
    let status = nssm_status(service_name);
    // For start/restart, the service must be running
    if (op == "start" || op == "restart") && status != "active" {
        return Err(anyhow::anyhow!(
            "Service started but stopped immediately (status: {status}).\n\
             Check the core's config or run status check for details."
        ));
    }
    // For stop, the service should be stopped
    if op == "stop" && status != "inactive" {
        return Err(anyhow::anyhow!(
            "Service stop command completed but status is still: {status}."
        ));
    }
    Ok(format!("{op}: {status}"))
}

// ============================================================================
// System proxy toggle (Windows registry)
// ============================================================================

const PROXY_REG_PATH: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";

/// Returns true if the system proxy is currently enabled.
pub fn get_system_proxy_state() -> Result<bool> {
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let settings = hkcu
        .open_subkey_with_flags(PROXY_REG_PATH, winreg::enums::KEY_READ)
        .map_err(|e| anyhow::anyhow!("Failed to open registry key: {e}"))?;
    let proxy_enable: u32 = settings
        .get_value("ProxyEnable")
        .map_err(|e| anyhow::anyhow!("Failed to read ProxyEnable: {e}"))?;
    Ok(proxy_enable == 1)
}

/// Enable system proxy on localhost:<port>.
pub fn enable_system_proxy(port: u16) -> Result<()> {
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let (settings, _) = hkcu
        .create_subkey(PROXY_REG_PATH)
        .map_err(|e| anyhow::anyhow!("Failed to open/create registry key: {e}"))?;
    settings
        .set_value("ProxyEnable", &1u32)
        .map_err(|e| anyhow::anyhow!("Failed to write ProxyEnable: {e}"))?;
    settings
        .set_value("ProxyServer", &format!("127.0.0.1:{port}"))
        .map_err(|e| anyhow::anyhow!("Failed to write ProxyServer: {e}"))?;
    settings
        .set_value("ProxyOverride", &"<-loopback>")
        .map_err(|e| anyhow::anyhow!("Failed to write ProxyOverride: {e}"))?;
    broadcast_settings_change();
    Ok(())
}

/// Disable system proxy.
pub fn disable_system_proxy() -> Result<()> {
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let (settings, _) = hkcu
        .create_subkey(PROXY_REG_PATH)
        .map_err(|e| anyhow::anyhow!("Failed to open/create registry key: {e}"))?;
    settings
        .set_value("ProxyEnable", &0u32)
        .map_err(|e| anyhow::anyhow!("Failed to write ProxyEnable: {e}"))?;
    broadcast_settings_change();
    Ok(())
}

/// Broadcast WM_SETTINGCHANGE so Windows refreshes proxy settings.
fn broadcast_settings_change() {
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        HWND_BROADCAST, SendMessageTimeoutW, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
    };
    let _ = unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            5000,
            None,
        )
    };
}

/// Retrieve the mixed inbound port from the core REST API (`GET /configs`).
pub fn get_mixed_port() -> Result<u16> {
    use crate::functions::restful::config_struct::ClashConfig;
    let resp = minreq::get(format!(
        "{}/configs",
        CONFIG.controller_for_core()
    ))
    .with_timeout(5)
    .send()
    .map_err(|e| anyhow::anyhow!("Failed to fetch /configs: {e}"))?;
    let cfg: ClashConfig =
        serde_json::from_str(resp.as_str().map_err(|e| anyhow::anyhow!("{e}"))?)
            .map_err(|e| anyhow::anyhow!("Failed to parse /configs: {e}"))?;
    cfg.mixed_port
        .or(cfg.port)
        .ok_or_else(|| anyhow::anyhow!("No mixed_port or port found in /configs"))
}
