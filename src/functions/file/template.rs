use super::{MAX_SUPPORTED_TEMPLATE_VERSION, PROFILE_YAMLS_PATH, TEMPLATE_PATH};
use crate::config::database::ProfileType;

mod version1;

pub fn get_all_templates() -> std::io::Result<Vec<String>> {
    Ok(std::fs::read_dir(TEMPLATE_PATH.as_path())?
        .collect::<std::io::Result<Vec<std::fs::DirEntry>>>()?
        .into_iter()
        .map(|p| {
            p.file_name()
                .into_string()
                .unwrap_or("Containing non UTF-8 char".to_owned())
        })
        .collect())
}
pub fn create_template(path: String) -> anyhow::Result<Option<String>> {
    let path = std::path::PathBuf::from(path);
    let file = std::fs::File::open(&path)?;
    let map: serde_yml::Mapping = serde_yml::from_reader(file)?;
    // remove extension if exists
    // file is opened, so file_name should exist
    let name = path.with_extension("").file_name().unwrap().to_owned();
    match map
        .get("clashtui_template_version")
        .and_then(|v| v.as_u64())
    {
        None => {
            std::fs::copy(&path, TEMPLATE_PATH.join(name))?;
            Ok(None)
        }
        Some(ver) if ver <= MAX_SUPPORTED_TEMPLATE_VERSION => {
            std::fs::copy(&path, TEMPLATE_PATH.join(&name))?;
            Ok(Some(format!(
                "Name:{} Added\nClashtui Template Version {}",
                // path from a String, should be UTF-8
                name.to_str().unwrap(),
                ver
            )))
        }
        Some(_) => anyhow::bail!(
            "Version higher than {} is not support",
            MAX_SUPPORTED_TEMPLATE_VERSION
        ),
    }
}
pub fn apply_template(template_name: &str, profile_name: &str) -> anyhow::Result<()> {
    let path = TEMPLATE_PATH.join(template_name);
    let file = std::fs::File::open(&path)
        .inspect_err(|e| log::error!("Founding template {template_name}:{e}"))?;
    let map: serde_yml::Mapping = serde_yml::from_reader(file)?;
    let gened = match map
        .get("clashtui_template_version")
        .and_then(|v| v.as_u64())
    {
        None | Some(1) => version1::gen_template(map, template_name)?,
        Some(_) => unimplemented!(),
    };
    let output_path = PROFILE_YAMLS_PATH.join(format!("{profile_name}.yaml"));
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    serde_yml::to_writer(std::fs::File::create(&output_path)?, &gened)?;
    let mut pm = pm!();
    pm.insert(profile_name, ProfileType::File);
    pm.to_file()?;
    Ok(())
}

const PROXY_PROVIDERS: &str = "proxy-providers";
const PROXY_GROUPS: &str = "proxy-groups";
const PROXIES: &str = "proxies";
const RULE_PROVIDERS: &str = "rule-providers";
const RULES: &str = "rules";

/// Remove net resource sections (`proxy-providers`, `rule-providers`) and embed
/// their remote content into the profile YAML. Also saves each downloaded
/// resource to the provider cache directory.
/// Downloads all resources in parallel via `spawn_blocking`.
/// Returns modified YAML mapping and per-resource update status.
pub async fn update_profile_without_pp(
    mut tpl: serde_yml::Mapping,
    with_proxy: bool,
) -> anyhow::Result<(serde_yml::Mapping, Vec<crate::functions::file::net_resource::NetResourceUpdate>)> {
    use crate::functions::file::net_resource::{NetResourceUpdate, ResourceSection};
    use std::collections::HashMap;

    let mut statuses: Vec<NetResourceUpdate> = Vec::new();

    // --- Proxy-Providers ---
    #[derive(serde::Deserialize, Debug)]
    struct PPitem {
        url: Option<String>,
        #[serde(flatten)]
        __others: serde_yml::Value,
    }
    #[derive(serde::Deserialize, serde::Serialize, Debug)]
    struct PGitem {
        #[serde(skip_serializing_if = "Option::is_none")]
        us_: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        proxies: Option<Vec<String>>,
        #[serde(flatten)]
        __others: serde_yml::Value,
    }

    let pp_proxies = if let Some(pps) = tpl.remove(PROXY_PROVIDERS) {
        let pps: HashMap<String, PPitem> = serde_yml::from_value(pps)?;

        let mut download_handles = Vec::new();
        for (pp_name, pp) in pps {
            let Some(url) = pp.url else { continue; };
            let pp_name_clone = pp_name.clone();
            let pp_path = pp
                .__others
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned();
            let cfg_dir = std::path::PathBuf::from(
                &crate::config::CONFIG.cfg_file.basic.clash_config_dir,
            );
            download_handles.push(tokio::task::spawn_blocking(move || {
                if !pp_path.is_empty() {
                    let dest = cfg_dir.join(&pp_path);
                    if let Ok(buf) = std::fs::read(&dest) {
                        if let Ok(yaml) = serde_yml::from_slice::<serde_yml::Mapping>(&buf) {
                            return (pp_name_clone, url, pp_path, Ok(yaml));
                        }
                    }
                }
                match crate::functions::restful::download::profile(&url, with_proxy) {
                    Ok(mut rdr) => {
                        let mut buf = Vec::new();
                        if let Err(e) = std::io::Read::read_to_end(&mut rdr, &mut buf) {
                            return (
                                pp_name_clone,
                                url,
                                pp_path,
                                Err(e.to_string()),
                            );
                        }
                        if !pp_path.is_empty() {
                            let dest = cfg_dir.join(&pp_path);
                            if let Some(parent) = dest.parent() {
                                let _ = std::fs::create_dir_all(parent);
                            }
                            let _ = std::fs::write(&dest, &buf);
                        }
                        let yaml = serde_yml::from_slice::<serde_yml::Mapping>(&buf).map_err(|e| e.to_string());
                        (pp_name_clone, url, pp_path, yaml)
                    }
                    Err(e) => (pp_name_clone, url, pp_path, Err(e.to_string())),
                }
            }));
        }

        let mut pp_proxies: HashMap<String, Vec<serde_yml::Value>> = HashMap::new();
        for handle in download_handles {
            let (pp_name, url, pp_path, result) = handle.await?;
            match result {
                Ok(mut loaded) => {
                    let loaded_proxies: Vec<serde_yml::Value> = loaded
                        .remove(PROXIES)
                        .and_then(|v| serde_yml::from_value(v).ok())
                        .unwrap_or_default();
                    let renamed_proxies = loaded_proxies
                        .into_iter()
                        .map(|mut proxy| {
                            if let Some(serde_yml::Value::String(name)) = proxy.get_mut("name") {
                                name.insert_str(0, pp_name.as_str());
                            }
                            proxy
                        })
                        .collect();
                    pp_proxies.insert(pp_name.clone(), renamed_proxies);
                    statuses.push(NetResourceUpdate {
                        name: pp_name,
                        url,
                        path: pp_path,
                        section: ResourceSection::ProxyProvider,
                        ok: true,
                        error: None,
                    });
                }
                Err(e) => {
                    statuses.push(NetResourceUpdate {
                        name: pp_name,
                        url,
                        path: pp_path,
                        section: ResourceSection::ProxyProvider,
                        ok: false,
                        error: Some(e),
                    });
                }
            }
        }
        pp_proxies
    } else {
        HashMap::new()
    };

    if !pp_proxies.is_empty() {
        let pgs = tpl
            .remove(PROXY_GROUPS)
            .ok_or(anyhow::anyhow!("{PROXY_GROUPS} not found"))?;
        let mut pgs: Vec<PGitem> = serde_yml::from_value(pgs)?;
        for pg in &mut pgs {
            let mut proxies = pg.proxies.take().unwrap_or_default();
            if let Some(uses) = pg.us_.take() {
                for pp_name in uses {
                    proxies.extend(
                        pp_proxies
                            .get(&pp_name)
                            .iter()
                            .flat_map(|v| v.iter())
                            .filter_map(|proxy| proxy.get("name"))
                            .map(|name| name.as_str().unwrap().to_owned()),
                    );
                }
            }
            if proxies.is_empty() {
                pg.proxies = Some(vec!["COMPATIBLE".to_owned()]);
            } else {
                pg.proxies = Some(proxies);
            }
        }
        tpl.insert(PROXY_GROUPS.into(), serde_yml::to_value(pgs)?);

        let mut tpl_proxies: Vec<serde_yml::Value> = tpl
            .remove(PROXIES)
            .and_then(|v| serde_yml::from_value(v).ok())
            .unwrap_or_default();
        tpl_proxies.extend(pp_proxies.into_values().flatten());
        tpl.insert(PROXIES.into(), tpl_proxies.into());
    }

    // --- Rule-Providers ---
    #[derive(serde::Deserialize, Debug)]
    struct RPitem {
        url: Option<String>,
        #[serde(flatten)]
        __others: serde_yml::Value,
    }

    if let Some(rps) = tpl.remove(RULE_PROVIDERS) {
        let rps: HashMap<String, RPitem> = serde_yml::from_value(rps)?;

        let mut download_handles = Vec::new();
        for (rp_name, rp) in rps {
            let Some(url) = rp.url else { continue; };
            let rp_name_clone = rp_name.clone();
            let rp_path = rp
                .__others
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned();
            let cfg_dir = std::path::PathBuf::from(
                &crate::config::CONFIG.cfg_file.basic.clash_config_dir,
            );
            download_handles.push(tokio::task::spawn_blocking(move || {
                if !rp_path.is_empty() {
                    let dest = cfg_dir.join(&rp_path);
                    if let Ok(buf) = std::fs::read(&dest) {
                        if let Ok(yaml) = serde_yml::from_slice::<serde_yml::Mapping>(&buf) {
                            return (rp_name_clone, url, rp_path, Ok(yaml));
                        }
                    }
                }
                match crate::functions::restful::download::profile(&url, with_proxy) {
                    Ok(mut rdr) => {
                        let mut buf = Vec::new();
                        if let Err(e) = std::io::Read::read_to_end(&mut rdr, &mut buf) {
                            return (
                                rp_name_clone,
                                url,
                                rp_path,
                                Err(e.to_string()),
                            );
                        }
                        if !rp_path.is_empty() {
                            let dest = cfg_dir.join(&rp_path);
                            if let Some(parent) = dest.parent() {
                                let _ = std::fs::create_dir_all(parent);
                            }
                            let _ = std::fs::write(&dest, &buf);
                        }
                        let yaml = serde_yml::from_slice::<serde_yml::Mapping>(&buf).map_err(|e| e.to_string());
                        (rp_name_clone, url, rp_path, yaml)
                    }
                    Err(e) => (rp_name_clone, url, rp_path, Err(e.to_string())),
                }
            }));
        }

        let mut all_rules: Vec<serde_yml::Value> = Vec::new();
        for handle in download_handles {
            let (rp_name, url, rp_path, result) = handle.await?;
            match result {
                Ok(mut loaded) => {
                    let rules: Vec<serde_yml::Value> = loaded
                        .remove("payload")
                        .or_else(|| loaded.remove(RULES))
                        .and_then(|v| serde_yml::from_value(v).ok())
                        .unwrap_or_default();
                    all_rules.extend(rules);
                    statuses.push(NetResourceUpdate {
                        name: rp_name,
                        url,
                        path: rp_path,
                        section: ResourceSection::RuleProvider,
                        ok: true,
                        error: None,
                    });
                }
                Err(e) => {
                    statuses.push(NetResourceUpdate {
                        name: rp_name,
                        url,
                        path: rp_path,
                        section: ResourceSection::RuleProvider,
                        ok: false,
                        error: Some(e),
                    });
                }
            }
        }
        if !all_rules.is_empty() {
            let mut existing_rules: Vec<serde_yml::Value> = tpl
                .remove(RULES)
                .and_then(|v| serde_yml::from_value(v).ok())
                .unwrap_or_default();
            existing_rules.extend(all_rules);
            tpl.insert(RULES.into(), existing_rules.into());
        }
    }

    Ok((tpl, statuses))
}

/// Extract net resource URLs from a YAML profile and download them in
/// parallel to collect status. Saves each downloaded resource to the
/// provider cache directory, keyed by its `path` field.
pub async fn fetch_net_resource_statuses(
    yaml: &serde_yml::Mapping,
    with_proxy: bool,
) -> Vec<crate::functions::file::net_resource::NetResourceUpdate> {
    use crate::functions::file::net_resource::{ExtractNetResources, NetResourceUpdate, ResourceSection};

    let resources =
        yaml.extract(&[ResourceSection::ProxyProvider, ResourceSection::RuleProvider]);

    if resources.is_empty() {
        return Vec::new();
    }

    let mut handles = Vec::with_capacity(resources.len());
    for resource in resources {
        let url = resource.url;
        let name = resource.name;
        let path = std::path::PathBuf::from(&crate::config::CONFIG.cfg_file.basic.clash_config_dir)
            .join(&resource.path);
        let section = resource.section;
        handles.push(tokio::task::spawn_blocking(move || {
            match crate::functions::restful::download::profile(&url, with_proxy) {
                Ok(mut rdr) => {
                    let mut buf = Vec::new();
                    if let Err(e) = std::io::Read::read_to_end(&mut rdr, &mut buf) {
                        return (name, url, path, section, false, Some(e.to_string()));
                    }
                    if let Some(parent) = path.parent() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            return (name, url, path, section, false, Some(e.to_string()));
                        }
                    }
                    match std::fs::write(&path, &buf) {
                        Ok(()) => (name, url, path, section, true, None),
                        Err(e) => (name, url, path, section, false, Some(e.to_string())),
                    }
                }
                Err(e) => (name, url, path, section, false, Some(e.to_string())),
            }
        }));
    }

    let mut statuses = Vec::with_capacity(handles.len());
    for handle in handles {
        let (name, url, path, section, ok, error) = match handle.await {
            Ok(v) => v,
            Err(e) => {
                statuses.push(NetResourceUpdate {
                    name: String::new(),
                    url: String::new(),
                    path: String::new(),
                    section: ResourceSection::ProxyProvider,
                    ok: false,
                    error: Some(e.to_string()),
                });
                continue;
            }
        };
        statuses.push(NetResourceUpdate {
            path: path.display().to_string(),
            name,
            url,
            section,
            ok,
            error,
        });
    }

    statuses
}
