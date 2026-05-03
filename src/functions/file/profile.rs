use std::fs::File;

mod profile;

use super::PROFILE_PATH;
use crate::config::database::{Profile, ProfileType};
pub use profile::LocalProfile;

pub mod db {
    use super::*;

    pub fn create(name: impl AsRef<str>, url: impl AsRef<str>) -> anyhow::Result<Profile> {
        let mut pm = pm!();
        pm.insert(&name, ProfileType::Url(url.as_ref().to_owned()));
        pm.to_file()?;
        Ok(pm.get(name).unwrap())
    }
    pub fn remove(pf: Profile) -> anyhow::Result<()> {
        if let Err(e) = std::fs::remove_file(PROFILE_PATH.join(&pf.name)) {
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
}

#[inline]
fn update_with<F: FnOnce(&str, bool) -> Result<minreq::ResponseLazy, minreq::Error>>(
    url: String,
    name: String,
    path: std::path::PathBuf,
    with_proxy: bool,
    apply: F,
) -> String {
    let url_domain = extract_domain(&url).unwrap_or("Unknown domain");
    match (|| -> anyhow::Result<()> {
        let mut response = apply(&url, with_proxy)?;
        // ensure a valid yaml content
        let content: serde_yml::Mapping = serde_yml::from_reader(&mut response)?;
        anyhow::ensure!(
            {
                content.get("proxies").is_some_and(|v| v.is_sequence())
                    || content
                        .get("proxy-providers")
                        .is_some_and(|v| v.is_mapping())
            },
            "Not a valid clash yaml file"
        );
        let output_file = File::create(path)?;
        serde_yml::to_writer(output_file, &content)?;
        Ok(())
    })() {
        // pretty output
        Ok(_) => format!("Updated: {name}({url_domain})"),
        Err(err) => {
            log::error!("Update profile {name}:{err}");
            format!("Not Updated: {name}({url_domain})")
        }
    }
}

pub async fn update_profile(
    profile: Profile,
    with_proxy: bool,
    remove_proxy_provider: bool,
) -> anyhow::Result<String> {
    let LocalProfile {
        name,
        dtype,
        path,
        content,
    } = profile.load_local_profile()?;
    // ensure path is valid
    anyhow::ensure!(
        path.is_absolute(),
        "trying to call `download_blob` without absolute path"
    );
    let directory = path
        .parent()
        .ok_or(anyhow::anyhow!("trying to download to '/' is not allowed"))?;
    if !directory.exists() {
        std::fs::create_dir_all(directory)?;
    }
    // do update
    let with_proxy = with_proxy
        && crate::functions::restful::control::version().is_ok()
        && crate::functions::restful::control::check_connectivity().is_ok();
    match dtype {
        // Imported file won't update, re-import and overwrite it if necessary
        ProfileType::File => anyhow::bail!("Not upgradable"),
        // Update via the given link
        ProfileType::Url(url) => {
            let res = update_with(url, name, path, with_proxy, |url, with_proxy| {
                crate::functions::restful::download::profile(url, with_proxy)
            });
            Ok(res)
        }
        ProfileType::Generated(template_name) => {
            // rebuild from template
            use super::template::apply_template;
            if let Err(e) = apply_template(template_name.clone()) {
                anyhow::bail!("Failed to regenerate from {template_name}: {e}")
            };
            let content = if remove_proxy_provider {
                use super::template::update_profile_without_pp;

                update_profile_without_pp(content.unwrap_or_default(), with_proxy)?
            } else {
                content.unwrap_or_default()
            };
            serde_yml::to_writer(std::fs::File::create(path)?, &content)?;
            Ok(format!("Regenerated: {}(From {template_name})", name))
        }
        ProfileType::Github { url, token } => {
            let res = update_with(url, name, path, with_proxy, |url, with_proxy| {
                crate::functions::restful::download::github(url, with_proxy, token)
            });
            Ok(res)
        }
        ProfileType::GitLab { url, token } => {
            let res = update_with(url, name, path, with_proxy, |url, with_proxy| {
                crate::functions::restful::download::gitlab(url, with_proxy, token)
            });
            Ok(res)
        }
    }
}

pub fn select(profile: Profile) -> anyhow::Result<()> {
    let cfg = &crate::config::CONFIG.cfg_file.basic;
    // load selected profile
    let mut lprofile = profile.clone().load_local_profile()?;
    // merge that into basic profile
    lprofile.merge(&crate::config::load_basic()?)?;
    // set path to clash config file path and sync to disk
    lprofile.path = cfg.clash_config_path.clone().into();
    lprofile.sync_to_disk()?;
    // after, change current profile
    db::set_current(profile)?;
    // ask clash to reload config
    crate::functions::restful::config::reload(&cfg.clash_config_path)?;
    Ok(())
}

pub fn extract_domain(url: &str) -> Option<&str> {
    if let Some(protocol_end) = url.find("://") {
        let rest = &url[(protocol_end + 3)..];
        return if let Some(path_start) = rest.find('/') {
            Some(&rest[..path_start])
        } else {
            Some(rest)
        };
    }
    None
}
