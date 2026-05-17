use super::*;
use crate::config::CoreType;
use anyhow::Result;
use std::path::PathBuf;

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

// ── Service operations via SCM API ─────────────────────────────

use windows_service::{
    service::{
        ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceType,
    },
    service_manager::{ServiceManager, ServiceManagerAccess},
};

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

/// Start or stop a Windows service.
pub fn windows_service_operation(op: &str, service_name: &str) -> Result<String> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;

    match op {
        "start" => {
            let service = manager.open_service(service_name, ServiceAccess::START)?;
            service.start(&[] as &[&str])?;
            Ok(format!("Service '{service_name}' started"))
        }
        "stop" => {
            let service = manager.open_service(service_name, ServiceAccess::STOP)?;
            service.stop()?;
            Ok(format!("Service '{service_name}' stopped"))
        }
        "restart" | "reload" => {
            // Best-effort stop, then start
            let _ = windows_service_operation("stop", service_name);
            windows_service_operation("start", service_name)
        }
        _ => Err(anyhow::anyhow!("Unknown Windows service operation: {op}")),
    }
}

/// Install a Windows service via SCM API.
pub fn windows_service_install(
    ct: CoreType,
    bin_path: &str,
    service_name: &str,
    launch_args: &[String],
) -> Result<String> {
    let manager_access = ServiceManagerAccess::CREATE_SERVICE;
    let manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_info = ServiceInfo {
        name: service_name.into(),
        display_name: match ct {
            CoreType::Mihomo => "ClashTui Mihomo".into(),
            CoreType::Singbox => "ClashTui SingBox".into(),
        },
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: bin_path.into(),
        launch_arguments: launch_args.iter().map(|s| s.as_str().into()).collect(),
        dependencies: vec![],
        account_name: None,
        account_password: None,
    };

    let _service =
        manager.create_service(&service_info, ServiceAccess::START | ServiceAccess::STOP)?;

    Ok(format!("Service '{service_name}' installed successfully"))
}

/// Uninstall a Windows service (stop first if running, then delete).
pub fn windows_service_uninstall(service_name: &str) -> Result<String> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;

    // Try to stop first if running
    if let Ok(service) =
        manager.open_service(service_name, ServiceAccess::STOP | ServiceAccess::QUERY_STATUS)
    {
        let status = service.query_status()?;
        if status.current_state != windows_service::service::ServiceState::Stopped {
            if let Err(e) = service.stop() {
                log::warn!("Failed to stop service '{service_name}' before uninstall: {e}");
            } else {
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        }
    }

    let service = manager.open_service(service_name, ServiceAccess::DELETE)?;
    service.delete()?;

    Ok(format!("Service '{service_name}' uninstalled"))
}

/// Query Windows service status.
pub fn windows_service_status(service_name: &str) -> String {
    let manager = match ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT) {
        Ok(m) => m,
        Err(_) => return "?".to_owned(),
    };

    let service = match manager.open_service(service_name, ServiceAccess::QUERY_STATUS) {
        Ok(s) => s,
        Err(_) => {
            // Service likely doesn't exist (not installed)
            return "uninstalled".to_owned();
        }
    };

    match service.query_status() {
        Ok(status) => {
            use windows_service::service::ServiceState;
            match status.current_state {
                ServiceState::Running => "active".to_owned(),
                ServiceState::Stopped
                | ServiceState::Paused
                | ServiceState::StartPending
                | ServiceState::StopPending
                | ServiceState::ContinuePending
                | ServiceState::PausePending => "inactive".to_owned(),
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
