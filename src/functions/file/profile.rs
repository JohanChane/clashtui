mod profile;

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
        if let Err(e) = std::fs::remove_file(PROFILE_YAMLS_PATH.join(format!("{}.yaml", &pf.name))) {
            if e.kind() != std::io::ErrorKind::NotFound {
                log::warn!("Failed to Remove profile file: {e}")
            }
        };
        let mut pm = pm!();
        pm.remove(pf.name);
        pm.to_file()
    }
    pub fn get(name: impl AsRef<str>) -> Option<Profile> {
        pm!().get(name)
    }
    pub fn get_all() -> Vec<Profile> {
        let pm = pm!();
        pm.all().into_iter().map(|k| pm.get(k).unwrap()).collect()
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

pub struct UpdateResult {
    pub name: String,
    pub net_updates: Vec<crate::functions::file::net_resource::NetResourceUpdate>,
}

pub async fn update_profile(
    profile: Profile,
    with_proxy: bool,
) -> anyhow::Result<UpdateResult> {
    use super::template::fetch_net_resource_statuses;

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

pub async fn select(profile: Profile) -> anyhow::Result<()> {
    use super::template::{fetch_net_resource_statuses, update_profile_without_pp};

    let cfg = &crate::config::CONFIG.cfg_file.basic;
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
    let out_path = std::path::absolute(std::path::PathBuf::from(&cfg.clash_config_path))
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
        &crate::config::CONFIG.cfg_file.basic.clash_config_dir,
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
