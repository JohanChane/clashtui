use std::fs::File;

mod profile;

use super::PROFILE_PATH;
use crate::config::database::{Profile, ProfileType};
use super::net_resource::{ExtractNetResources, ResourceSection};
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
    clash_cfg_dir: &std::path::Path,
    apply: F,
) -> Vec<String> {
    let url_domain = extract_domain(&url).unwrap_or("Unknown domain");
    let mut results: Vec<String> = Vec::new();

    let content: serde_yml::Mapping = match (|| -> anyhow::Result<serde_yml::Mapping> {
        let mut response = apply(&url, with_proxy)?;
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
        let output_file = File::create(&path)?;
        serde_yml::to_writer(output_file, &content)?;
        Ok(content)
    })() {
        Ok(content) => {
            results.push(format!("Updated: {name}({url_domain})"));
            content
        }
        Err(err) => {
            let msg = format!("Not Updated: {name}({url_domain}): {err}");
            log::error!("{msg}");
            results.push(msg);
            return results;
        }
    };

    let net_resources = content.extract(&[ResourceSection::ProxyProvider, ResourceSection::RuleProvider]);
    for res in &net_resources {
        let res_domain = extract_domain(&res.url).unwrap_or("Unknown domain");
        let res_path = clash_cfg_dir.join(&res.path);
        if let Some(parent) = res_path.parent() {
            if !parent.exists() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    results.push(format!("  Not Updated: {}({}): create dir error: {e}", res.name, res_domain));
                    continue;
                }
            }
        }
        match crate::functions::restful::download::profile(&res.url, with_proxy) {
            Ok(mut response) => {
                let output_file = match File::create(&res_path) {
                    Ok(f) => f,
                    Err(e) => {
                        results.push(format!("  Not Updated: {}({}): {e}", res.name, res_domain));
                        continue;
                    }
                };
                let mut buf_writer = std::io::BufWriter::new(output_file);
                match std::io::copy(&mut response, &mut buf_writer) {
                    Ok(_) => results.push(format!("  Updated: {}({})", res.name, res_domain)),
                    Err(e) => results.push(format!("  Not Updated: {}({}): write error: {e}", res.name, res_domain)),
                }
            }
            Err(e) => {
                results.push(format!("  Not Updated: {}({}): {e}", res.name, res_domain));
            }
        }
    }

    results
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
    let clash_cfg_dir = std::path::Path::new(&crate::config::CONFIG.cfg_file.basic.clash_config_dir);
    match dtype {
        // Imported file won't update, re-import and overwrite it if necessary
        ProfileType::File => anyhow::bail!("Not upgradable"),
        // Update via the given link
        ProfileType::Url(url) => {
            let lines = update_with(url, name, path, with_proxy, clash_cfg_dir, |url, with_proxy| {
                crate::functions::restful::download::profile(url, with_proxy)
            });
            Ok(lines.join("\n"))
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
    }
}

pub fn select(profile: Profile) -> anyhow::Result<()> {
    let cfg = &crate::config::CONFIG.cfg_file.basic;
    let mut lprofile = profile.clone().load_local_profile()?;
    lprofile.merge(&crate::config::load_basic()?)?;
    lprofile.path = cfg.clash_config_path.clone().into();
    lprofile.sync_to_disk()?;
    db::set_current(profile)?;
    if let Err(e) = crate::functions::restful::config::reload(&cfg.clash_config_path) {
        log::warn!("Failed to reload clash config: {e}");
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
