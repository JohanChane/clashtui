use crate::config::{CONFIG, CoreType};
use crate::functions::restful::config_struct::ClashConfig;
use anyhow::{Context, Result};
use std::path::Path;

/// Persist only the runtime fields editable in Settings, preserving the rest.
pub fn save() -> Result<()> {
    update_config(|content, core| {
        let runtime = crate::functions::restful::config::fetch()
            .context("Failed to fetch runtime settings")?;
        merge_settings(content, core, &runtime)
    })
}

fn update_config(update: impl FnOnce(&str, CoreType) -> Result<String>) -> Result<()> {
    static SAVE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = SAVE_LOCK.lock().expect("config save lock poisoned");
    anyhow::ensure!(!crate::config::is_core_mismatch(), "core mismatch");
    let core = CONFIG.core_type();
    let path = match core {
        CoreType::Mihomo => &CONFIG.cfg_file.mihomo.core.config_path,
        CoreType::Singbox => &CONFIG.cfg_file.singbox.core.config_path,
    };
    anyhow::ensure!(!path.is_empty(), "Core config_path is empty");
    let path = Path::new(path);
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let output = update(&content, core)?;
    atomic_write(path, output.as_bytes())
}

pub fn save_log_level(level: &str) -> Result<()> {
    update_config(|content, core| match core {
        CoreType::Mihomo => {
            let mut config: serde_yml::Mapping = serde_yml::from_str(content)?;
            config.insert("log-level".into(), level.into());
            Ok(serde_yml::to_string(&config)?)
        }
        CoreType::Singbox => {
            let mut config: serde_json::Map<String, serde_json::Value> =
                serde_json::from_str(content)?;
            let log = config.entry("log").or_insert_with(|| serde_json::json!({}));
            let log = log.as_object_mut().context("log must be an object")?;
            log.insert("disabled".to_owned(), (level == "silent").into());
            if level != "silent" {
                let level = if level == "warning" { "warn" } else { level };
                log.insert("level".to_owned(), level.into());
            }
            Ok(serde_json::to_string_pretty(&config)? + "\n")
        }
    })
}

pub fn save_proxy_selections(
    selections: &std::collections::BTreeMap<String, String>,
) -> Result<()> {
    update_config(|content, core| {
        let live = crate::functions::restful::proxies::fetch_proxies()
            .context("Failed to verify current proxy selections")?;
        for (name, node) in selections {
            anyhow::ensure!(
                live.primary
                    .get(name)
                    .and_then(|group| group.now.as_deref())
                    == Some(node.as_str()),
                "Selection for {name} has changed in the core. Select a node again before saving"
            );
        }
        match core {
            CoreType::Mihomo => {
                let mut config: serde_yml::Mapping = serde_yml::from_str(content)?;
                let groups = config
                    .get_mut("proxy-groups")
                    .and_then(serde_yml::Value::as_sequence_mut)
                    .context("No proxy groups in the core config")?;
                for (name, node) in selections {
                    let group = groups
                        .iter_mut()
                        .find(|group| {
                            group["name"].as_str() == Some(name.as_str())
                                && group["type"].as_str() == Some("select")
                        })
                        .with_context(|| format!("{name} is not a configured select group"))?;
                    group["default-selected"] = node.as_str().into();
                }
                Ok(serde_yml::to_string(&config)?)
            }
            CoreType::Singbox => {
                let mut config: serde_json::Value = serde_json::from_str(content)?;
                let outbounds = config
                    .get_mut("outbounds")
                    .and_then(serde_json::Value::as_array_mut)
                    .context("No outbounds in the core config")?;
                for (name, node) in selections {
                    let group = outbounds
                        .iter_mut()
                        .find(|group| {
                            group["tag"].as_str() == Some(name.as_str())
                                && group["type"].as_str() == Some("selector")
                        })
                        .with_context(|| format!("{name} is not a configured selector"))?;
                    anyhow::ensure!(
                        group["outbounds"].as_array().is_some_and(|items| items
                            .iter()
                            .any(|item| item.as_str() == Some(node.as_str()))),
                        "{node} is not in selector {name}"
                    );
                    group["default"] = node.as_str().into();
                }
                Ok(serde_json::to_string_pretty(&config)? + "\n")
            }
        }
    })
}

fn merge_settings(content: &str, core: CoreType, runtime: &ClashConfig) -> Result<String> {
    match core {
        CoreType::Mihomo => {
            let mut value: serde_yml::Value = serde_yml::from_str(content)?;
            let map = value
                .as_mapping_mut()
                .context("Core YAML must be a mapping")?;
            map.insert(
                "mode".into(),
                runtime.mode.to_string().to_lowercase().into(),
            );
            if let Some(allow) = runtime.allow_lan {
                map.insert("allow-lan".into(), allow.into());
            }
            if let Some(tun) = &runtime.tun {
                let tun_value = map
                    .entry(serde_yml::Value::from("tun"))
                    .or_insert_with(|| serde_yml::Value::Mapping(Default::default()));
                let tun_map = tun_value
                    .as_mapping_mut()
                    .context("tun must be a mapping")?;
                tun_map.insert("enable".into(), tun.enable.into());
                tun_map.insert("stack".into(), tun.stack.to_string().to_lowercase().into());
            }
            Ok(serde_yml::to_string(&value)?)
        }
        CoreType::Singbox => {
            let mut value: serde_json::Value = serde_json::from_str(content)?;
            anyhow::ensure!(value.is_object(), "Core JSON must be an object");
            // sing-box uses its native schema, not the Clash API response schema.
            let root = value
                .as_object_mut()
                .context("Core JSON must be an object")?;
            let experimental = root
                .entry("experimental")
                .or_insert_with(|| serde_json::json!({}));
            let experimental = experimental
                .as_object_mut()
                .context("experimental must be an object")?;
            let api = experimental
                .entry("clash_api")
                .or_insert_with(|| serde_json::json!({}));
            let api = api
                .as_object_mut()
                .context("experimental.clash_api must be an object")?;
            api.insert(
                "default_mode".to_owned(),
                runtime.mode.to_string().to_lowercase().into(),
            );
            if let Some(allow) = runtime.allow_lan
                && let Some(inbounds) = value.get_mut("inbounds").and_then(|v| v.as_array_mut())
            {
                for inbound in inbounds {
                    if matches!(inbound["type"].as_str(), Some("mixed" | "http" | "socks")) {
                        inbound["listen"] = if allow { "0.0.0.0" } else { "127.0.0.1" }.into();
                    }
                }
            }
            Ok(serde_json::to_string_pretty(&value)? + "\n")
        }
    }
}

fn atomic_write(path: &Path, content: &[u8]) -> Result<()> {
    use std::io::Write;
    let temporary = path.with_file_name(format!(".clashtui-save-{:016x}.tmp", fastrand::u64(..)));
    let result = (|| -> Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(content)?;
        file.set_permissions(std::fs::metadata(path)?.permissions())?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result.with_context(|| format!("Failed to save {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime() -> ClashConfig {
        serde_json::from_value(serde_json::json!({
            "mode": "global", "allow-lan": true,
            "tun": {"enable": true, "stack": "System"}
        }))
        .unwrap()
    }

    #[test]
    fn mihomo_preserves_unedited_fields_and_nested_tun_options() {
        let output = merge_settings(
            "mode: rule\ntun:\n  enable: false\n  auto-route: true\nrules:\n  - MATCH,DIRECT\n",
            CoreType::Mihomo,
            &runtime(),
        )
        .unwrap();
        let value: serde_yml::Value = serde_yml::from_str(&output).unwrap();
        assert_eq!(value["mode"].as_str(), Some("global"));
        assert_eq!(value["allow-lan"].as_bool(), Some(true));
        assert_eq!(value["tun"]["stack"].as_str(), Some("system"));
        assert_eq!(value["tun"]["auto-route"].as_bool(), Some(true));
        assert_eq!(value["rules"][0].as_str(), Some("MATCH,DIRECT"));
    }

    #[test]
    fn singbox_uses_native_keys_and_preserves_api_secret_and_tun() {
        let content = r#"{"experimental":{"clash_api":{"secret":"keep"}},"inbounds":[{"type":"mixed","listen":"127.0.0.1","listen_port":2080},{"type":"tun","address":["172.18.0.1/30"]}]}"#;
        let value: serde_json::Value =
            serde_json::from_str(&merge_settings(content, CoreType::Singbox, &runtime()).unwrap())
                .unwrap();
        assert_eq!(value["experimental"]["clash_api"]["default_mode"], "global");
        assert_eq!(value["experimental"]["clash_api"]["secret"], "keep");
        assert_eq!(value["inbounds"][0]["listen"], "0.0.0.0");
        assert_eq!(value["inbounds"][1]["address"][0], "172.18.0.1/30");
        assert!(value.get("mode").is_none());
    }

    #[test]
    fn malformed_config_is_rejected() {
        assert!(merge_settings("[", CoreType::Mihomo, &runtime()).is_err());
        assert!(merge_settings("[]", CoreType::Singbox, &runtime()).is_err());
        assert!(
            merge_settings(r#"{"experimental":false}"#, CoreType::Singbox, &runtime()).is_err()
        );
    }

    #[test]
    fn atomic_write_replaces_existing_file() {
        let path =
            std::env::temp_dir().join(format!("clashtui-save-test-{:016x}", fastrand::u64(..)));
        std::fs::write(&path, "old").unwrap();
        atomic_write(&path, b"new").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
        std::fs::remove_file(path).unwrap();
    }
}
