//! Internal core configuration checks. These do not use script actions.
use crate::{
    command,
    config::{CONFIG, CoreType},
};
use anyhow::{Result, ensure};
use std::{ffi::OsString, path::Path, sync::atomic::AtomicBool};

pub fn test_config(
    profile_path: Option<&Path>,
    enable_geodata_mode: bool,
    cancel: &AtomicBool,
) -> Result<String> {
    let cfg = &CONFIG.cfg_file.mihomo.core;
    let mut args: Vec<OsString> = vec![
        "-t".into(),
        "-d".into(),
        cfg.config_dir.clone().into(),
        "-f".into(),
        profile_path
            .unwrap_or_else(|| Path::new(&cfg.config_path))
            .into(),
    ];
    if enable_geodata_mode {
        args.push("-m".into());
    }
    let output = command::capture(cfg.bin_path.as_ref(), &args, cancel)?;
    Ok(format!(
        "{}\n{}",
        if output.status.success() {
            "OK".into()
        } else {
            format!("Error({})", output.status)
        },
        output.captured.diagnostic()
    ))
}

pub fn check_config(core: CoreType, profile_path: &Path, cancel: &AtomicBool) -> Result<()> {
    let output = match core {
        CoreType::Mihomo => {
            let cfg = &CONFIG.cfg_file.mihomo.core;
            command::capture(
                cfg.bin_path.as_ref(),
                &[
                    "-t".into(),
                    "-d".into(),
                    cfg.config_dir.clone().into(),
                    "-f".into(),
                    profile_path.into(),
                ],
                cancel,
            )?
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

            let output = command::capture(
                cfg.bin_path.as_ref(),
                &[
                    "check".into(),
                    "-D".into(),
                    cfg.config_dir.clone().into(),
                    "-c".into(),
                    check_path.as_os_str().into(),
                ],
                cancel,
            );
            if check_path != profile_path {
                let _ = std::fs::remove_file(&check_path);
            }
            output?
        }
    };
    ensure!(
        output.status.success(),
        "{core} configuration check failed ({}):\n{}",
        output.status,
        output.captured.diagnostic()
    );
    Ok(())
}
pub fn restart_service() -> Result<String> {
    super::service::execute_current("restart_core")
}
pub fn stop_service() -> Result<String> {
    super::service::execute_current("stop_core")
}
