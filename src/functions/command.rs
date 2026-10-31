#[cfg(unix)]
mod unix;
mod utils;

use crate::config::CONFIG;
use crate::config::CoreType;
use anyhow::Result;
use std::{path::Path, process::Command};

#[cfg(unix)]
pub use unix::*;
pub(crate) use utils::sanitize_windows_path;
use utils::*;

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
                        .is_some_and(|obj| obj.remove("clashtui").is_some())
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

pub fn restart_service() -> Result<String> {
    super::service::execute_current("restart_core")
}

pub fn stop_service() -> Result<String> {
    super::service::execute_current("stop_core")
}

pub fn edit(path: &str) -> Result<()> {
    let tpl = CONFIG.cfg_file.extra.edit_cmd.as_deref().unwrap_or("");
    log::debug!("edit: path={path} template={tpl}");
    shell_spawn(tpl, path)
}

pub fn open_dir(path: &Path) -> Result<()> {
    let tpl = CONFIG.cfg_file.extra.open_dir_cmd.as_deref().unwrap_or("");
    let path = path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("directory path is not valid UTF-8"))?;
    log::debug!("open_dir: path={path} template={tpl}");
    shell_spawn(tpl, path)
}
