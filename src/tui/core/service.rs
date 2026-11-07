use super::{ServiceObservation, ServiceOperation};
use crate::config::{CONFIG, CoreType, ServiceController};
use std::{process::Command, time::Duration};

/// Service probes are finite and own the child until it exits.
fn output(mut command: Command) -> Result<(bool, String, String), String> {
    let output = crate::functions::command::probe(&mut command, Duration::from_secs(5))
        .map_err(|e| format!("{e:#}"))?;
    Ok((
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).trim().into(),
        String::from_utf8_lossy(&output.stderr).trim().into(),
    ))
}

pub(super) fn observe(core: CoreType) -> Result<ServiceObservation, String> {
    let config = match core {
        CoreType::Mihomo => &CONFIG.cfg_file.mihomo.core_service,
        CoreType::Singbox => &CONFIG.cfg_file.singbox.core_service,
    };
    let name = if config.service_name.is_empty() {
        match core {
            CoreType::Mihomo => "clashtui_mihomo",
            CoreType::Singbox => "clashtui_singbox",
        }
    } else {
        &config.service_name
    };
    let controller = ServiceController::from_config(config);
    let mut command;
    match controller {
        ServiceController::Nssm => {
            command = Command::new(controller.bin_name());
            command.args(["status", name]);
        }
        ServiceController::Launchd => {
            command = Command::new("launchctl");
            #[cfg(unix)]
            let domain = if config.is_user {
                format!("gui/{}", unsafe { libc::getuid() })
            } else {
                "system".into()
            };
            #[cfg(not(unix))]
            let domain = "system".to_owned();
            command.args(["print", &format!("{domain}/{name}")]);
        }
        ServiceController::OpenRc => {
            command = Command::new(controller.bin_name());
            command.args([name, "status"]);
        }
        _ => {
            command = Command::new("systemctl");
            command.arg("is-active");
            if config.is_user {
                command.arg("--user");
            }
            command.arg(name);
        }
    }
    let (success, stdout, stderr) = output(command)?;
    let running = match controller {
        ServiceController::Nssm if stdout.contains("SERVICE_RUNNING") => Some(true),
        ServiceController::Nssm
            if stdout.contains("SERVICE_STOPPED") || stdout.contains("SERVICE_PAUSED") =>
        {
            Some(false)
        }
        ServiceController::Launchd if success => Some(stdout.contains("state = running")),
        ServiceController::OpenRc if stdout.contains("started") => Some(true),
        ServiceController::OpenRc if stdout.contains("stopped") => Some(false),
        ServiceController::Systemd if stdout == "active" => Some(true),
        ServiceController::Systemd
            if matches!(stdout.as_str(), "inactive" | "failed" | "deactivating") =>
        {
            Some(false)
        }
        _ => None,
    };
    let description = match running {
        Some(true) => "active".into(),
        Some(false) => "inactive".into(),
        None => {
            if stderr.is_empty() {
                if stdout.is_empty() {
                    "Unknown service state".into()
                } else {
                    stdout
                }
            } else {
                format!("{stdout} {stderr}").trim().to_owned()
            }
        }
    };
    Ok(ServiceObservation {
        running,
        description,
    })
}

pub(super) fn execute(core: CoreType, operation: ServiceOperation) -> Result<String, String> {
    use crate::functions::command;
    let result = match operation {
        ServiceOperation::Start => command::start_core_service(core),
        ServiceOperation::Restart => command::restart_service(),
        ServiceOperation::Stop => command::stop_core_service(core),
        ServiceOperation::StopAll => command::stop_all_services(),
        ServiceOperation::SwitchCore => {
            let stopped = command::stop_all_services();
            CONFIG.data.lock().unwrap().core_type = core;
            CONFIG.save().map_err(|e| e.to_string())?;
            let started = command::start_core_service(core);
            let message = format!(
                "Core switched to {core}\n\nStop all: {}\nStart: {}",
                stopped.unwrap_or_else(|e| e.to_string()),
                started.unwrap_or_else(|e| e.to_string())
            );
            let _ = crate::tui::popmsg::prelude::Confirm::dismiss_any("Core Switched".into())
                .with_prompt(message.clone())
                .build_and_send()
                .blocking_recv();
            crate::tui::app::request_quit();
            Ok(message)
        }
        #[cfg(windows)]
        ServiceOperation::Install => command::install_core_service(core),
        #[cfg(windows)]
        ServiceOperation::Uninstall => command::uninstall_core_service(core),
    };
    result.map_err(|e| format!("{e:#}"))
}
