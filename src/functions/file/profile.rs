mod profile;

use super::PROFILE_JSONS_PATH;
use super::PROFILE_YAMLS_PATH;
use crate::config::database::{Profile, ProfileType};

pub mod db {
    use super::*;

    pub fn create(name: impl AsRef<str>, url: impl AsRef<str>) -> anyhow::Result<Profile> {
        let mut pm = pm!();
        pm.insert(&name, ProfileType::Url(url.as_ref().to_owned()));
        pm.to_file()?;
        Ok(pm.get(name).unwrap())
    }
    pub fn remove(pf: Profile) -> anyhow::Result<()> {
        for path in [
            PROFILE_JSONS_PATH.join(format!("{}.json", &pf.name)),
            PROFILE_YAMLS_PATH.join(format!("{}.yaml", &pf.name)),
        ] {
            if let Err(e) = std::fs::remove_file(&path) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    log::warn!("Failed to Remove profile file {}: {e}", path.display());
                }
            }
        }
        let mut pm = pm!();
        pm.remove(pf.name);
        pm.to_file()
    }
    pub fn get(name: impl AsRef<str>) -> Option<Profile> {
        pm!().get(name)
    }
    pub fn get_all() -> Vec<Profile> {
        let pm = pm!();
        pm.all_for_core()
            .into_iter()
            .map(|k| pm.get(k).unwrap())
            .collect()
    }
    pub fn get_current() -> Profile {
        pm!().get_current().unwrap_or_default()
    }
    pub fn set_current(pf: Profile) -> anyhow::Result<()> {
        let mut pm = pm!();
        pm.set_current(pf);
        pm.to_file()
    }
    pub fn toggle_no_pp(name: impl AsRef<str>) -> anyhow::Result<bool> {
        let mut pm = pm!();
        let current = pm.get(name.as_ref()).map(|pf| pf.no_pp).unwrap_or(false);
        let new = !current;
        pm.set_no_pp(name.as_ref(), new);
        pm.to_file()?;
        Ok(new)
    }
    pub fn toggle_update_with_proxy(name: impl AsRef<str>) -> anyhow::Result<bool> {
        let mut pm = pm!();
        let current = pm.get(name.as_ref()).map(|pf| pf.update_with_proxy).unwrap_or(false);
        let new = !current;
        pm.set_update_with_proxy(name.as_ref(), new);
        pm.to_file()?;
        Ok(new)
    }
}

pub fn import_profile_from_file(source_path: &str, profile_name: &str) -> anyhow::Result<Profile> {
    let source = std::path::Path::new(source_path);
    anyhow::ensure!(source.exists(), "Source file not found: {source_path}");
    anyhow::ensure!(source.is_file(), "Source path is not a file: {source_path}");

    let is_json = source
        .extension()
        .map(|e| e.eq_ignore_ascii_case("json"))
        .unwrap_or(false);

    if is_json {
        return import_singbox_profile(source, profile_name);
    }

    let content: serde_yml::Mapping = {
        let file = std::fs::File::open(source)?;
        serde_yml::from_reader(file)
            .map_err(|e| anyhow::anyhow!("Invalid YAML in source file: {e}"))?
    };
    anyhow::ensure!(
        content.get("proxies").is_some_and(|v| v.is_sequence())
            || content
                .get("proxy-providers")
                .is_some_and(|v| v.is_mapping()),
        "Not a valid clash YAML file"
    );

    let dest = PROFILE_YAMLS_PATH.join(format!("{profile_name}.yaml"));
    if dest.exists() {
        anyhow::bail!("Profile '{profile_name}' already exists in profile_yamls/");
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(source, &dest)?;

    let mut pm = pm!();
    pm.insert(profile_name, ProfileType::File);
    pm.to_file()?;
    Ok(pm.get(profile_name).unwrap())
}

fn import_singbox_profile(source: &std::path::Path, profile_name: &str) -> anyhow::Result<Profile> {
    let file = std::fs::File::open(source)?;
    let content: serde_json::Value = serde_json::from_reader(file)
        .map_err(|e| anyhow::anyhow!("Invalid JSON in source file: {e}"))?;

    anyhow::ensure!(
        content.get("outbounds").is_some_and(|v| v.is_array()),
        "Not a valid sing-box JSON profile (missing 'outbounds' array)"
    );

    if let Some(parent) = PROFILE_JSONS_PATH.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir_all(&*PROFILE_JSONS_PATH)?;

    let dest = PROFILE_JSONS_PATH.join(format!("{profile_name}.json"));
    if dest.exists() {
        anyhow::bail!("Profile '{profile_name}' already exists in profile_jsons/");
    }
    std::fs::copy(source, &dest)?;

    let mut pm = pm!();
    pm.insert(profile_name, ProfileType::Singbox);
    pm.to_file()?;
    Ok(pm.get(profile_name).unwrap())
}

pub struct UpdateResult {
    pub name: String,
    pub net_updates: Vec<crate::functions::file::net_resource::NetResourceUpdate>,
}

pub async fn update_profile(profile: Profile, with_proxy: bool) -> anyhow::Result<UpdateResult> {
    use super::template::fetch_net_resource_statuses;

    let result = if matches!(profile.dtype, ProfileType::Template { .. }) {
        update_template_profile(profile.clone(), with_proxy).await
    } else if matches!(profile.dtype, ProfileType::Singbox)
        || crate::config::CONFIG.core_type() == crate::config::CoreType::Singbox
    {
        update_singbox_profile(profile.clone(), with_proxy).await
    } else {
        let path = PROFILE_YAMLS_PATH.join(format!("{}.yaml", &profile.name));

        if let ProfileType::Url(ref url) = profile.dtype {
            let mut response = crate::functions::restful::download::profile(url, with_proxy)?;
            let content: serde_yml::Mapping = serde_yml::from_reader(&mut response)
                .map_err(|e| anyhow::anyhow!("Failed to parse downloaded profile YAML: {e}"))?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            serde_yml::to_writer(std::fs::File::create(&path)?, &content)?;
        }

        anyhow::ensure!(
            path.exists(),
            "Profile file not found: {}. Download it first.",
            path.display()
        );

        let content: serde_yml::Mapping = {
            let file = std::fs::File::open(&path)?;
            serde_yml::from_reader(file)
                .map_err(|e| anyhow::anyhow!("Failed to read profile YAML: {e}"))?
        };

        let net_updates = fetch_net_resource_statuses(&content, with_proxy).await;
        serde_yml::to_writer(std::fs::File::create(&path)?, &content)?;
        Ok(UpdateResult {
            name: profile.name.clone(),
            net_updates,
        })
    };

    if result.is_ok() {
        let cur = db::get_current();
        if cur.name == profile.name {
            let _ = select(profile).await;
        }
    }

    result
}

async fn update_singbox_profile(
    profile: Profile,
    with_proxy: bool,
) -> anyhow::Result<UpdateResult> {
    let path = PROFILE_JSONS_PATH.join(format!("{}.json", &profile.name));

    if let ProfileType::Url(ref url) = profile.dtype {
        let mut response = crate::functions::restful::download::profile(url, with_proxy)?;
        let content: serde_json::Value = serde_json::from_reader(&mut response)
            .map_err(|e| anyhow::anyhow!("Failed to parse downloaded profile JSON: {e}"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::create(&path)?;
        serde_json::to_writer_pretty(file, &content)?;
    }

    anyhow::ensure!(
        path.exists(),
        "Profile file not found: {}. Download it first.",
        path.display()
    );

    let content: serde_json::Value = {
        let file = std::fs::File::open(&path)?;
        serde_json::from_reader(file)
            .map_err(|e| anyhow::anyhow!("Failed to read profile JSON: {e}"))?
    };

    let net_resources =
        crate::functions::file::net_resource::extract_singbox_net_resources(&content);
    let base_dir = std::path::Path::new(&crate::config::CONFIG.cfg_file.singbox.core.config_dir);
    let net_updates = crate::functions::file::template::fetch_net_resource_statuses_from_resources(
        &net_resources,
        base_dir,
        with_proxy,
    )
    .await;

    Ok(UpdateResult {
        name: profile.name,
        net_updates,
    })
}

async fn update_template_profile(
    profile: Profile,
    with_proxy: bool,
) -> anyhow::Result<UpdateResult> {
    use crate::functions::file::net_resource::{
        ExtractNetResources, NetResourceUpdate, ResourceSection,
    };

    let template = match &profile.dtype {
        ProfileType::Template { template } => template.clone(),
        _ => anyhow::bail!("update_template_profile called on non-Template profile"),
    };

    // Read proxy-provider URLs from the generated profile file
    let groups = super::template::read_profile_ppg(&profile.name).unwrap_or_default();

    let is_singbox = crate::config::CONFIG.core_type() == crate::config::CoreType::Singbox;
    let mut statuses: Vec<NetResourceUpdate> = Vec::new();

    if is_singbox {
        // For sing-box, download proxy-provider subscription content to proxy-providers dir
        let mut download_handles = Vec::new();
        for providers in groups.values() {
            for (name, url) in providers {
                let url = url.clone();
                let name = name.clone();
                let hash = format!("{:x}", md5::compute(url.as_bytes()));
                let path =
                    crate::config::singbox_proxy_providers_path().join(format!("{hash}.json"));
                download_handles.push(tokio::task::spawn_blocking(move || {
                    match crate::functions::restful::download::profile(&url, with_proxy) {
                        Ok(mut rdr) => {
                            let mut buf = Vec::new();
                            if let Err(e) = std::io::Read::read_to_end(&mut rdr, &mut buf) {
                                return (name, url, path, false, Some(e.to_string()));
                            }
                            if let Some(parent) = path.parent() {
                                if let Err(e) = std::fs::create_dir_all(parent) {
                                    return (name, url, path, false, Some(e.to_string()));
                                }
                            }
                            match std::fs::write(&path, &buf) {
                                Ok(()) => (name, url, path, true, None),
                                Err(e) => (name, url, path, false, Some(e.to_string())),
                            }
                        }
                        Err(e) => {
                            if path.exists() {
                                (name, url, path, true, None)
                            } else {
                                (name, url, path, false, Some(e.to_string()))
                            }
                        }
                    }
                }));
            }
        }

        for handle in download_handles {
            let (name, url, path, ok, error) = handle.await?;
            statuses.push(NetResourceUpdate {
                name,
                url,
                path: path.display().to_string(),
                section: ResourceSection::ProxyProvider,
                ok,
                error,
            });
        }
    } else {
        let cfg_dir =
            std::path::PathBuf::from(&crate::config::CONFIG.cfg_file.mihomo.core.config_dir);
        let _tpl_name = std::path::Path::new(&template)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&template);

        // Collect URLs from both proxy-provider groups and standalone providers.
        // Standalone proxy-providers (with own `url`, no `tpl_param`) are not in
        // clashtui.proxy_provider_groups but need to be pre-downloaded as well.
        let mut download_urls: Vec<(String, String)> = Vec::new();
        for providers in groups.values() {
            for (name, url) in providers {
                download_urls.push((name.clone(), url.clone()));
            }
        }

        // Also extract standalone proxy-provider URLs from the generated profile
        let profile_path = super::PROFILE_YAMLS_PATH.join(format!("{}.yaml", &profile.name));
        if let Ok(content) = std::fs::read_to_string(&profile_path) {
            if let Ok(mapping) = serde_yml::from_str::<serde_yml::Mapping>(&content) {
                for resource in mapping.extract(&[ResourceSection::ProxyProvider]) {
                    let already_in_groups = groups
                        .values()
                        .flat_map(|providers| providers.values())
                        .any(|url| url == &resource.url);
                    if !already_in_groups {
                        download_urls.push((resource.name, resource.url));
                    }
                }
            }
        }

        let mut download_handles = Vec::new();
        for (name, url) in download_urls {
            let url = url.clone();
            let name = name.clone();
            let hash = format!("{:x}", md5::compute(url.as_bytes()));
            let path = cfg_dir.join(format!("proxies/{hash}"));
            download_handles.push(tokio::task::spawn_blocking(move || {
                match crate::functions::restful::download::profile(&url, with_proxy) {
                    Ok(mut rdr) => {
                        let mut buf = Vec::new();
                        if let Err(e) = std::io::Read::read_to_end(&mut rdr, &mut buf) {
                            return (name, url, path, false, Some(e.to_string()));
                        }
                        if serde_yml::from_slice::<serde_yml::Mapping>(&buf).is_err() {
                            return (
                                name,
                                url,
                                path,
                                false,
                                Some("Invalid YAML format".to_string()),
                            );
                        }
                        if let Some(parent) = path.parent() {
                            if let Err(e) = std::fs::create_dir_all(parent) {
                                return (name, url, path, false, Some(e.to_string()));
                            }
                        }
                        match std::fs::write(&path, &buf) {
                            Ok(()) => (name, url, path, true, None),
                            Err(e) => (name, url, path, false, Some(e.to_string())),
                        }
                    }
                    Err(e) => {
                        if path.exists()
                            && std::fs::read(&path).is_ok_and(|buf| {
                                serde_yml::from_slice::<serde_yml::Mapping>(&buf).is_ok()
                            })
                        {
                            (name, url, path, true, None)
                        } else {
                            (name, url, path, false, Some(e.to_string()))
                        }
                    }
                }
            }));
        }

        let mut all_ok = true;
        for handle in download_handles {
            let (name, url, path, ok, error) = handle.await?;
            if !ok {
                all_ok = false;
            }
            statuses.push(NetResourceUpdate {
                name,
                url,
                path: path.display().to_string(),
                section: ResourceSection::ProxyProvider,
                ok,
                error,
            });
        }

        if !all_ok {
            let failures: Vec<String> = statuses
                .iter()
                .filter(|s| !s.ok)
                .map(|s| {
                    format!(
                        "  {}: {} — {}",
                        s.name,
                        extract_domain(&s.url).unwrap_or(&s.url),
                        s.error.as_deref().unwrap_or("unknown error")
                    )
                })
                .collect();
            anyhow::bail!(
                "Failed to download proxy providers:\n{}",
                failures.join("\n")
            );
        }
    }

    Ok(UpdateResult {
        name: profile.name,
        net_updates: statuses,
    })
}

pub async fn select(profile: Profile) -> anyhow::Result<()> {
    use super::template::{
        check_template_ppg_availability, fetch_net_resource_statuses, update_profile_without_pp,
    };

    // For Template profiles, verify proxy-provider files exist before selection
    if matches!(profile.dtype, ProfileType::Template { .. }) {
        check_template_ppg_availability(&profile)?;
    }

    if matches!(profile.dtype, ProfileType::Singbox)
        || crate::config::CONFIG.core_type() == crate::config::CoreType::Singbox
    {
        return select_singbox(profile).await;
    }

    let cfg = &crate::config::CONFIG.cfg_file.mihomo.core;
    let mut lprofile = profile.clone().load_local_profile()?;
    anyhow::ensure!(
        lprofile.content.is_some(),
        "Profile {} is empty or not yet downloaded. Run update first.",
        profile.name
    );

    if profile.no_pp {
        let content = lprofile.content.take().unwrap_or_default();
        let (new_content, _) = update_profile_without_pp(content, false).await?;
        lprofile.content = Some(new_content);
    } else if let Some(ref content) = lprofile.content {
        fetch_net_resource_statuses(content, false).await;
    }

    rewrite_provider_paths(lprofile.content.as_mut());

    lprofile.merge(&crate::config::load_basic()?)?;
    // Strip clashtui metadata before writing to core config
    if let Some(ref mut content) = lprofile.content {
        content.remove("clashtui");
    }
    let out_path = std::path::absolute(std::path::PathBuf::from(&cfg.config_path))
        .map_err(|e| anyhow::anyhow!("Failed to resolve config path: {e}"))?;
    lprofile.path = out_path.clone();
    lprofile.sync_to_disk()?;
    db::set_current(profile)?;
    crate::functions::restful::config::reload(&out_path.display().to_string())
        .map_err(|e| anyhow::anyhow!("Config written but reload failed: {e}"))?;
    Ok(())
}

fn rewrite_provider_paths(_content: Option<&mut serde_yml::Mapping>) {
    // Paths are kept as-is (relative to mihomo's -d working directory).
    // Mihomo resolves relative proxy-provider/rule-provider paths against
    // its config directory, avoiding hard-coded absolute paths that break
    // when config_dir changes (e.g. switching between user/system mode).
}

fn deep_merge(base: &mut serde_json::Value, overlay: &serde_json::Value) {
    let serde_json::Value::Object(base_map) = base else {
        *base = overlay.clone();
        return;
    };
    let serde_json::Value::Object(overlay_map) = overlay else {
        *base = overlay.clone();
        return;
    };
    for (key, value) in overlay_map {
        match base_map.get_mut(key.as_str()) {
            Some(base_value) => deep_merge(base_value, value),
            None => {
                base_map.insert(key.clone(), value.clone());
            }
        }
    }
}

async fn select_singbox(profile: Profile) -> anyhow::Result<()> {
    let profile_path = super::PROFILE_JSONS_PATH.join(format!("{}.json", &profile.name));
    anyhow::ensure!(
        profile_path.exists(),
        "Profile {} file not found: {}. Download it first.",
        profile.name,
        profile_path.display()
    );

    let cfg = &crate::config::CONFIG.cfg_file.singbox.core;
    let override_path = crate::config::singbox_core_override_path();

    let out_path = std::path::absolute(std::path::PathBuf::from(&cfg.config_path))
        .map_err(|e| anyhow::anyhow!("Failed to resolve singbox config path: {e}"))?;

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let profile_content = std::fs::read_to_string(&profile_path)
        .map_err(|e| anyhow::anyhow!("Failed to read profile {}: {e}", profile_path.display()))?;
    let mut config: serde_json::Value = serde_json::from_str(&profile_content)
        .map_err(|e| anyhow::anyhow!("Failed to parse profile {}: {e}", profile_path.display()))?;

    // Strip clashtui metadata before merging into core config
    if let Some(obj) = config.as_object_mut() {
        obj.remove("clashtui");
    }

    if override_path.exists() {
        let override_content = std::fs::read_to_string(&override_path)
            .map_err(|e| anyhow::anyhow!("Failed to read core_override_config.json: {e}"))?;
        let overlay: serde_json::Value = serde_json::from_str(&override_content)
            .map_err(|e| anyhow::anyhow!("Failed to parse core_override_config.json: {e}"))?;
        deep_merge(&mut config, &overlay);
    } else {
        log::warn!(
            "core_override_config.json not found at {}, using profile as-is",
            override_path.display()
        );
    }

    let merged_content = serde_json::to_string_pretty(&config)
        .map_err(|e| anyhow::anyhow!("Failed to serialize merged config: {e}"))?;
    std::fs::write(&out_path, merged_content)
        .map_err(|e| anyhow::anyhow!("Failed to write config to {}: {e}", out_path.display()))?;

    db::set_current(profile)?;

    let is_user = crate::config::CONFIG.cfg_file.singbox.core_service.is_user;
    let needs_sudo = !is_user;

    #[cfg(feature = "tui")]
    let password = crate::functions::command::resolve_sudo_password(needs_sudo)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    #[cfg(not(feature = "tui"))]
    let password: Option<String> = None;

    let reload_out = crate::functions::command::reload_core_service(
        password.as_deref(),
        crate::config::CoreType::Singbox,
    )
    .map_err(|e| anyhow::anyhow!("Config written but service reload failed: {e}"))?;
    if reload_out.starts_with("Error") {
        return Err(anyhow::anyhow!("Service reload failed:\n{reload_out}"));
    }
    Ok(())
}

pub fn extract_domain(url: &str) -> Option<&str> {
    if let Some(protocol_end) = url.find("://") {
        let rest = &url[(protocol_end + 3)..];
        let rest = if let Some(at_pos) = rest.find('@') {
            if let Some(slash_pos) = rest.find('/') {
                if at_pos < slash_pos {
                    &rest[(at_pos + 1)..]
                } else {
                    rest
                }
            } else {
                &rest[(at_pos + 1)..]
            }
        } else {
            rest
        };
        return if let Some(path_start) = rest.find('/') {
            Some(&rest[..path_start])
        } else {
            Some(rest)
        };
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn merge(base_json: &str, overlay_json: &str) -> serde_json::Value {
        let mut base: serde_json::Value = serde_json::from_str(base_json).unwrap();
        let overlay: serde_json::Value = serde_json::from_str(overlay_json).unwrap();
        deep_merge(&mut base, &overlay);
        base
    }

    #[test]
    fn scalar_overwrite() {
        let result = merge(r#"{"port": 7890}"#, r#"{"port": 20122}"#);
        assert_eq!(result["port"], 20122);
    }

    #[test]
    fn object_recursive_merge() {
        let result = merge(
            r#"{"experimental": {"clash_api": {"external_controller": "0.0.0.0:9090"}}}"#,
            r#"{"experimental": {"clash_api": {"secret": "abc"}}}"#,
        );
        assert_eq!(
            result["experimental"]["clash_api"]["external_controller"],
            "0.0.0.0:9090"
        );
        assert_eq!(result["experimental"]["clash_api"]["secret"], "abc");
    }

    #[test]
    fn array_replaced_entirely() {
        let result = merge(
            r#"{"inbounds": [{"type": "mixed", "port": 7890}, {"type": "http", "port": 8080}]}"#,
            r#"{"inbounds": [{"type": "tun", "stack": "gvisor"}]}"#,
        );
        let inbounds = result["inbounds"].as_array().unwrap();
        assert_eq!(inbounds.len(), 1);
        assert_eq!(inbounds[0]["type"], "tun");
    }

    #[test]
    fn overlay_adds_new_top_level_key() {
        let result = merge(
            r#"{"route": {"final": "proxy"}}"#,
            r#"{"log": {"level": "debug"}}"#,
        );
        assert_eq!(result["route"]["final"], "proxy");
        assert_eq!(result["log"]["level"], "debug");
    }

    #[test]
    fn base_only_keys_preserved() {
        let result = merge(
            r#"{"route": {"rules": [], "final": "proxy"}, "dns": {}}"#,
            r#"{"log": {"level": "info"}}"#,
        );
        assert!(result["route"]["rules"].is_array());
        assert_eq!(result["route"]["final"], "proxy");
        assert!(!result["dns"].is_null()); // empty object preserved
        assert_eq!(result["log"]["level"], "info");
    }

    #[test]
    fn overlay_object_overwrites_base_scalar() {
        let result = merge(r#"{"log": "info"}"#, r#"{"log": {"level": "debug"}}"#);
        assert_eq!(result["log"]["level"], "debug");
    }

    #[test]
    fn overlay_scalar_overwrites_base_object() {
        let result = merge(
            r#"{"experimental": {"clash_api": {"port": 9090}}}"#,
            r#"{"experimental": "disabled"}"#,
        );
        assert_eq!(result["experimental"], "disabled");
    }

    #[test]
    fn empty_overlay_is_noop() {
        let result = merge(r#"{"port": 7890, "tun": {"stack": "system"}}"#, r#"{}"#);
        assert_eq!(result["port"], 7890);
        assert_eq!(result["tun"]["stack"], "system");
    }

    #[test]
    fn deep_nested_merge() {
        let result = merge(
            r#"{"a": {"b": {"c": 1, "d": 2}}}"#,
            r#"{"a": {"b": {"c": 10, "e": 3}}}"#,
        );
        assert_eq!(result["a"]["b"]["c"], 10); // overwritten
        assert_eq!(result["a"]["b"]["d"], 2); // preserved
        assert_eq!(result["a"]["b"]["e"], 3); // added
    }

    #[test]
    fn array_in_nested_object_is_replaced() {
        let result = merge(
            r#"{"a": {"b": [1, 2, 3], "c": "keep"}}"#,
            r#"{"a": {"b": [4, 5]}}"#,
        );
        let b = result["a"]["b"].as_array().unwrap();
        assert_eq!(b.len(), 2);
        assert_eq!(b[0], 4);
        assert_eq!(b[1], 5);
        assert_eq!(result["a"]["c"], "keep");
    }

    #[test]
    fn base_empty_with_overlay() {
        let result = merge(
            r#"{}"#,
            r#"{"inbounds": [{"type": "mixed", "port": 20122}], "log": {"level": "info"}}"#,
        );
        assert_eq!(result["inbounds"].as_array().unwrap().len(), 1);
        assert_eq!(result["log"]["level"], "info");
    }

    // ── Tests for standalone proxy-provider URL extraction during update ──────

    use crate::config::database::ProxyProviderGroups;
    use crate::functions::file::net_resource::{ExtractNetResources, ResourceSection};
    use std::collections::BTreeMap;

    /// Collect all proxy-provider download URLs from groups + generated profile,
    /// with deduplication (same logic as in `update_template_profile`).
    fn collect_proxy_provider_urls(
        profile_yaml: &str,
        groups: &ProxyProviderGroups,
    ) -> Vec<(String, String)> {
        let mut urls: Vec<(String, String)> = Vec::new();
        for providers in groups.values() {
            for (name, url) in providers {
                urls.push((name.clone(), url.clone()));
            }
        }

        if let Ok(mapping) = serde_yml::from_str::<serde_yml::Mapping>(profile_yaml) {
            for resource in mapping.extract(&[ResourceSection::ProxyProvider]) {
                let already_in_groups = groups
                    .values()
                    .flat_map(|providers| providers.values())
                    .any(|url| url == &resource.url);
                if !already_in_groups {
                    urls.push((resource.name, resource.url));
                }
            }
        }
        urls
    }

    #[test]
    fn standalone_proxy_provider_url_collected() {
        let profile_yaml = r#"
proxy-providers:
  pvd0:
    type: http
    url: https://example.com/sub1.yaml
    interval: 3600
  bak:
    type: http
    url: https://hajimi.nvimy.com/file/bak.yaml
    interval: 3600
proxy-groups:
  - name: "Entry"
    type: select
    use:
      - pvd0
  - name: "Special"
    type: select
    use:
      - bak
"#;

        let mut providers = BTreeMap::new();
        providers.insert(
            "pvd0".to_string(),
            "https://example.com/sub1.yaml".to_string(),
        );
        let mut groups = ProxyProviderGroups::new();
        groups.insert("pvd".to_string(), providers);

        let urls = collect_proxy_provider_urls(profile_yaml, &groups);

        // Group provider (pvd0) should be included
        assert!(
            urls.iter().any(|(name, _)| name == "pvd0"),
            "pvd0 from groups should be collected"
        );
        // Standalone provider (bak) should also be collected
        assert!(
            urls.iter().any(|(name, _)| name == "bak"),
            "bak standalone provider should be collected"
        );

        // pvd0 should appear only once (not duplicated from extract)
        let pvd0_count = urls.iter().filter(|(name, _)| name == "pvd0").count();
        assert_eq!(pvd0_count, 1, "pvd0 should not be duplicated");
    }

    #[test]
    fn standalone_proxy_provider_bak_specifically_collected() {
        // Realistic generated profile mimicking the user's setup:
        // two group-expanded providers (hajimi, mojie) + standalone bak
        let profile_yaml = r#"
proxy-providers:
  hajimi:
    type: http
    url: https://hajimi.nvimy.com/file/clash.yaml
  mojie:
    type: http
    url: https://hajimi.nvimy.com/file/clash_mojie.yaml
  bak:
    type: http
    url: https://hajimi.nvimy.com/file/mojie_johan.yaml
    override:
      additional-prefix: '[bak]'
proxy-groups:
  - name: "Entry"
    type: select
    use:
      - hajimi
      - mojie
  - name: "Special"
    type: select
    use:
      - bak
"#;

        let mut pvd_providers = BTreeMap::new();
        pvd_providers.insert(
            "hajimi".to_string(),
            "https://hajimi.nvimy.com/file/clash.yaml".to_string(),
        );
        pvd_providers.insert(
            "mojie".to_string(),
            "https://hajimi.nvimy.com/file/clash_mojie.yaml".to_string(),
        );
        let mut groups = ProxyProviderGroups::new();
        groups.insert("pvd".to_string(), pvd_providers);

        let urls = collect_proxy_provider_urls(profile_yaml, &groups);

        assert_eq!(
            urls.len(),
            3,
            "should collect 3 unique URLs: hajimi, mojie, bak"
        );

        assert!(urls.iter().any(|(name, _)| name == "hajimi"));
        assert!(urls.iter().any(|(name, _)| name == "mojie"));
        assert!(
            urls.iter().any(|(name, _)| name == "bak"),
            "bak MUST be collected as standalone provider"
        );

        // Verify bak's URL is correct
        let bak_url = urls
            .iter()
            .find(|(name, _)| name == "bak")
            .map(|(_, url)| url);
        assert_eq!(
            bak_url,
            Some(&"https://hajimi.nvimy.com/file/mojie_johan.yaml".to_string())
        );
    }

    #[test]
    fn empty_groups_standalone_still_collected() {
        // When groups are empty (no tpl_param providers), standalone ones still work
        let profile_yaml = r#"
proxy-providers:
  bak:
    type: http
    url: https://example.com/standalone.yaml
proxy-groups:
  - name: "Entry"
    type: select
    use:
      - bak
"#;
        let groups = ProxyProviderGroups::new();

        let urls = collect_proxy_provider_urls(profile_yaml, &groups);
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].0, "bak");
        assert!(urls[0].1.contains("standalone.yaml"));
    }

    #[test]
    fn no_standalone_when_all_in_groups() {
        let profile_yaml = r#"
proxy-providers:
  pvd0:
    type: http
    url: https://example.com/sub1.yaml
  pvd1:
    type: http
    url: https://example.com/sub2.yaml
proxy-groups:
  - name: "Entry"
    type: select
    use:
      - pvd0
      - pvd1
"#;
        let mut providers = BTreeMap::new();
        providers.insert(
            "pvd0".to_string(),
            "https://example.com/sub1.yaml".to_string(),
        );
        providers.insert(
            "pvd1".to_string(),
            "https://example.com/sub2.yaml".to_string(),
        );
        let mut groups = ProxyProviderGroups::new();
        groups.insert("pvd".to_string(), providers);

        let urls = collect_proxy_provider_urls(profile_yaml, &groups);
        assert_eq!(urls.len(), 2, "only the two group providers, no duplicates");
    }
}
