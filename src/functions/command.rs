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

pub fn restart_service() -> Result<String> {
    let host = &CONFIG.cfg_file.hack.service_controller;
    let svc = &CONFIG.cfg_file.service;

    exec(
        host.bin_name(),
        host.args("restart", &svc.clash_service_name, svc.is_user),
    )?;
    exec(
        host.bin_name(),
        host.args("status", &svc.clash_service_name, svc.is_user),
    )
}

pub fn stop_service() -> Result<String> {
    let host = &CONFIG.cfg_file.hack.service_controller;
    let svc = &CONFIG.cfg_file.service;

    exec(
        host.bin_name(),
        host.args("stop", &svc.clash_service_name, svc.is_user),
    )?;
    exec(
        host.bin_name(),
        host.args("status", &svc.clash_service_name, svc.is_user),
    )
}

pub fn edit(path: &str) -> Result<()> {
    spawn(
        "sh",
        vec!["-c", CONFIG.cfg_file.edit_cmd.replace("%s", path).as_str()],
    )
}
