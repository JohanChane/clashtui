#[cfg_attr(target_os = "linux", path = "command/linux.rs")]
mod platform;
mod utils;

use crate::config::CONFIG;
use crate::config::CoreType;
use anyhow::Result;
use std::{path::Path, process::Command};

pub use platform::*;
use utils::*;

pub fn test_config(profile_path: Option<&Path>, enable_geodata_mode: bool) -> String {
    let cfg = &CONFIG.cfg_file.basic;

    let mut cmd = Command::new(&cfg.clash_bin_path);
    cmd.args(["-t", "-d", &cfg.clash_config_dir, "-f"]);
    if let Some(path) = profile_path {
        cmd.arg(path);
    } else {
        cmd.arg(&cfg.clash_config_path);
    }

    if enable_geodata_mode {
        cmd.arg("-m");
    }

    let opt = cmd.output().unwrap();
    stringify_output(opt)
}

fn svc_operation(op: &str, password: Option<&str>, core_type: Option<CoreType>) -> Result<String> {
    let host = &CONFIG.cfg_file.hack.service_controller;
    let svc = &CONFIG.cfg_file.service;
    let ct = core_type.unwrap_or(CONFIG.cfg_file.core_type);

    let (service_name, is_user) = match ct {
        CoreType::Mihomo => (&svc.clash_service_name, svc.is_user),
        CoreType::Singbox => (&svc.singbox_service_name, svc.singbox_is_user),
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

pub fn restart_service(password: Option<&str>) -> Result<String> {
    svc_operation("restart", password, None)
}

pub fn stop_service(password: Option<&str>) -> Result<String> {
    svc_operation("stop", password, None)
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
