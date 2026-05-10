mod profile;

use super::PROFILE_YAMLS_PATH;
use super::PROFILE_JSONS_PATH;
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
        pm.all_for_core().into_iter().map(|k| pm.get(k).unwrap()).collect()
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
        anyhow::bail!(
            "Profile '{profile_name}' already exists in profile_yamls/"
        );
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
        anyhow::bail!(
            "Profile '{profile_name}' already exists in profile_jsons/"
        );
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

pub async fn update_profile(
    profile: Profile,
    with_proxy: bool,
) -> anyhow::Result<UpdateResult> {
    use super::template::fetch_net_resource_statuses;

    // Template profiles re-generate from template + subscriptions
    if matches!(profile.dtype, ProfileType::Template { .. }) {
        return update_template_profile(profile, with_proxy).await;
    }

    // sing-box local imports always use JSON; URL profiles follow current core type
    if matches!(profile.dtype, ProfileType::Singbox)
        || crate::config::CONFIG.core_type() == crate::config::CoreType::Singbox
    {
        return update_singbox_profile(profile, with_proxy).await;
    }

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
        name: profile.name,
        net_updates,
    })
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
    let base_dir = std::path::Path::new(
        &crate::config::CONFIG.cfg_file.singbox.core.config_dir,
    );
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
    use crate::functions::file::net_resource::{NetResourceUpdate, ResourceSection};

    let (template, groups) = match &profile.dtype {
        ProfileType::Template { template, proxy_provider_groups } => (template.clone(), proxy_provider_groups.clone()),
        _ => anyhow::bail!("update_template_profile called on non-Template profile"),
    };

    let is_singbox = crate::config::CONFIG.core_type() == crate::config::CoreType::Singbox;
    let mut statuses: Vec<NetResourceUpdate> = Vec::new();

    if is_singbox {
        super::template::apply_template_singbox(&template, &profile.name, &groups, with_proxy, true).await?;
        for (_, providers) in &groups {
            for (name, url) in providers {
                let domain = extract_domain(url).unwrap_or("unknown");
                statuses.push(NetResourceUpdate {
                    name: name.clone(),
                    url: url.clone(),
                    path: String::new(),
                    section: ResourceSection::ProxyProvider,
                    ok: true,
                    error: None,
                });
            }
        }
    } else {
        let cfg_dir = std::path::PathBuf::from(
            &crate::config::CONFIG.cfg_file.mihomo.core.config_dir,
        );
        let tpl_name = std::path::Path::new(&template)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&template);

        let mut download_handles = Vec::new();
        for providers in groups.values() {
            for (name, url) in providers {
                let url = url.clone();
                let name = name.clone();
                let path = cfg_dir.join(format!("proxy-providers/tpl/{}/{}.yaml", tpl_name, &name));
                download_handles.push(tokio::task::spawn_blocking(move || {
                    match crate::functions::restful::download::profile(&url, with_proxy) {
                        Ok(mut rdr) => {
                            let mut buf = Vec::new();
                            if let Err(e) = std::io::Read::read_to_end(&mut rdr, &mut buf) {
                                return (name, url, path, false, Some(e.to_string()));
                            }
                            if serde_yml::from_slice::<serde_yml::Mapping>(&buf).is_err() {
                                return (name, url, path, false, Some("Invalid YAML format".to_string()));
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
                            if path.exists() && std::fs::read(&path).is_ok_and(|buf| {
                                serde_yml::from_slice::<serde_yml::Mapping>(&buf).is_ok()
                            }) {
                                (name, url, path, true, None)
                            } else {
                                (name, url, path, false, Some(e.to_string()))
                            }
                        }
                    }
                }));
            }
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
                "Failed to download proxy providers — profile not generated:\n{}",
                failures.join("\n")
            );
        }

        super::template::apply_template(&template, &profile.name, &groups)?;
    }

    Ok(UpdateResult {
        name: profile.name,
        net_updates: statuses,
    })
}

pub async fn select(profile: Profile) -> anyhow::Result<()> {
    use super::template::{fetch_net_resource_statuses, update_profile_without_pp};

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
    let out_path = std::path::absolute(std::path::PathBuf::from(&cfg.config_path))
        .map_err(|e| anyhow::anyhow!("Failed to resolve config path: {e}"))?;
    lprofile.path = out_path.clone();
    lprofile.sync_to_disk()?;
    db::set_current(profile)?;
    crate::functions::restful::config::reload(&out_path.display().to_string())
        .map_err(|e| anyhow::anyhow!("Config written but reload failed: {e}"))?;
    Ok(())
}

fn rewrite_provider_paths(content: Option<&mut serde_yml::Mapping>) {
    let Some(content) = content else { return };
    let cache = std::path::PathBuf::from(
        &crate::config::CONFIG.cfg_file.mihomo.core.config_dir,
    );
    for section in &["proxy-providers", "rule-providers"] {
        let Some(serde_yml::Value::Mapping(providers)) = content.get_mut(*section) else {
            continue;
        };
        for (_, v) in providers {
            let Some(provider) = v.as_mapping_mut() else { continue };
            let Some(path_val) = provider.get_mut("path") else { continue };
            let Some(rel) = path_val.as_str() else { continue };
            let abs_path = cache.join(rel);
            *path_val = serde_yml::Value::String(abs_path.display().to_string());
        }
    }
}

async fn select_singbox(profile: Profile) -> anyhow::Result<()> {
    let path = super::PROFILE_JSONS_PATH.join(format!("{}.json", &profile.name));
    anyhow::ensure!(
        path.exists(),
        "Profile {} file not found: {}. Download it first.",
        profile.name, path.display()
    );

    let mut profile_content: serde_json::Value = {
        let file = std::fs::File::open(&path)?;
        serde_json::from_reader(file)
            .map_err(|e| anyhow::anyhow!("Failed to read profile JSON: {e}"))?
    };

    match crate::config::load_basic_singbox() {
        Ok(core_override) => {
            if let (Some(profile_obj), Some(override_obj)) =
                (profile_content.as_object_mut(), core_override.as_object())
            {
                for (key, value) in override_obj {
                    profile_obj.insert(key.clone(), value.clone());
                }
            }
        }
        Err(e) => {
            log::warn!("Failed to load core override singbox config: {e}, using profile as-is");
        }
    }

    let out_path = std::path::absolute(std::path::PathBuf::from(
        &crate::config::CONFIG.cfg_file.singbox.core.config_path,
    ))
    .map_err(|e| anyhow::anyhow!("Failed to resolve singbox config path: {e}"))?;

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::File::create(&out_path)?;
    serde_json::to_writer(file, &profile_content)?;

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
                if at_pos < slash_pos { &rest[(at_pos + 1)..] } else { rest }
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
