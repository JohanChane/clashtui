#[cfg_attr(target_os = "linux", path = "command/linux.rs")]
mod platform;
mod utils;

use crate::config::CONFIG;
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

fn svc_operation(op: &str, password: Option<&str>) -> Result<String> {
    let host = &CONFIG.cfg_file.hack.service_controller;
    let svc = &CONFIG.cfg_file.service;

    let svc_args = host.args(op, &svc.clash_service_name, svc.is_user);
    if svc.is_user {
        return exec(host.bin_name(), svc_args);
    }
    let mut argv = vec![host.bin_name()];
    argv.extend(svc_args);

    match password {
        Some(pw) => exec_sudo(argv, pw),
        None => exec("sudo", argv),
    }
}

pub fn restart_service(password: Option<&str>) -> Result<String> {
    svc_operation("restart", password)
}

pub fn stop_service(password: Option<&str>) -> Result<String> {
    svc_operation("stop", password)
}

pub fn set_permission(bin_path: &str, password: Option<&str>) -> Result<String> {
    let setcap_path = Path::new("/usr/sbin/setcap");
    let setcap_bin = if setcap_path.exists() {
        setcap_path.as_os_str().to_str().unwrap_or("setcap")
    } else {
        "setcap"
    };
    let cap_args = "cap_net_admin,cap_net_bind_service=+ep";

    let real_path = std::fs::canonicalize(bin_path)
        .unwrap_or_else(|_| Path::new(bin_path).to_path_buf());
    let real_path_str = real_path.to_str().unwrap_or(bin_path);

    let argv = vec![setcap_bin, cap_args, real_path_str];

    match password {
        Some(pw) => exec_sudo(argv, pw),
        None => exec("sudo", argv),
    }
}

pub fn edit(path: &str) -> Result<()> {
    spawn(
        "sh",
        vec!["-c", CONFIG.cfg_file.edit_cmd.replace("%s", path).as_str()],
    )
}
