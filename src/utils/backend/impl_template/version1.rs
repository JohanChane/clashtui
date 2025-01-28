use std::collections::HashMap;

use crate::BackEnd;

const PROXY_PROVIDERS: &str = "proxy-providers";
const PROXY_GROUPS: &str = "proxy-groups";
const PROXIES: &str = "proxies";

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
struct PGparam {
    /// create one [PGitem] for each related providers,
    /// with name remap to `{name}-{provider_name}`
    ///
    /// e.g. `At-pvd0`
    ///
    /// these are actually prefixs
    providers: Vec<String>,
}
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
struct PGitem {
    /// maybe generated
    name: String,
    #[serde(rename = "use")]
    #[serde(skip_serializing_if = "Option::is_none")]
    /// maybe generated
    us_: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// maybe generated
    proxies: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// will be removed, not in final output
    tpl_param: Option<PGparam>,
    #[serde(rename = "type")]
    /// not cared, just keep this
    __type: String,
    /// not cared, just keep this
    #[serde(flatten)]
    __others: serde_yml::Value,
}
#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct PPitem {
    /// maybe generated
    url: Option<String>,
    /// maybe generated
    path: Option<String>,
    #[serde(flatten)]
    /// may contain `tpl_param` as maker,
    /// remove that before final output
    others: serde_yml::Mapping,
    #[serde(rename = "type")]
    /// not cared, just keep this
    __type: String,
}

pub(super) fn gen_template(
    mut tpl: serde_yml::Mapping,
    tpl_name: &str,
    local_urls: Vec<String>,
) -> anyhow::Result<serde_yml::Mapping> {
    // proxy-groups
    let pgs = tpl
        .remove(PROXY_GROUPS)
        .ok_or(anyhow::anyhow!("{PROXY_GROUPS} not found"))?;
    let pgs: Vec<PGitem> = serde_yml::from_value(pgs)?;
    // proxy-providers
    let pps = tpl
        .remove(PROXY_PROVIDERS)
        .ok_or(anyhow::anyhow!("{PROXY_PROVIDERS} not found"))?;
    let pps: HashMap<String, PPitem> = serde_yml::from_value(pps)?;
    // proxy-providers name as key, proxy-providers as value
    let mut extended_proxy_providers = HashMap::new();
    let mut extended_proxy_groups = Vec::new();

    for (pp_name, mut pp) in pps {
        // remove marker
        if pp.others.remove("tpl_param").is_some() {
            // in this iteration, pp is read only after url and path are set
            // so cloning is not needed
            for (idx, url) in local_urls.iter().enumerate() {
                // let mut pp = pp.clone();
                let proxy_provider_name = format!("{pp_name}{idx}");
                pp.url = Some(url.clone());
                pp.path = Some(format!(
                    "proxy-providers/tpl/{tpl_name}/{proxy_provider_name}.yaml"
                ));
                extended_proxy_providers.insert(proxy_provider_name, serde_yml::to_value(&pp)?);
            }
        } else {
            extended_proxy_providers.insert(pp_name, serde_yml::to_value(&pp)?);
        }
    }
    // list to handle 'proxies' section
    let mut proxies_to_do = vec![];
    for mut pg in pgs {
        let ref_mut_vec = if pg
            .proxies
            .as_ref()
            .is_some_and(|v| v.iter().any(|s| s.starts_with('<') && s.ends_with('>')))
        {
            &mut proxies_to_do
        } else {
            &mut extended_proxy_groups
        };
        if let Some(param) = pg.tpl_param.take() {
            let PGparam { providers } = param;
            let pg_name = std::mem::take(&mut pg.name);
            for pp_prefix in providers {
                for pp_name in extended_proxy_providers.keys() {
                    if pp_name.starts_with(&pp_prefix) {
                        let mut new_pg = pg.clone();
                        new_pg.name = format!("{pg_name}-{pp_name}");
                        new_pg.us_ = Some(vec![pp_name.clone()]);
                        ref_mut_vec.push(new_pg);
                    }
                }
            }
        } else {
            ref_mut_vec.push(pg);
        }
    }

    for mut pg in proxies_to_do {
        let proxies = pg.proxies.take().unwrap();
        let mut keep_list: Vec<String> = proxies
            .iter()
            .filter(|s| !(s.starts_with('<') && s.ends_with('>')))
            .cloned()
            .collect();
        let rebind_list: Vec<String> = proxies
            .into_iter()
            .filter(|s| s.starts_with('<') && s.ends_with('>'))
            .map(|s| s.chars().skip(1).take(s.len() - 2).collect())
            .collect();
        let proxies = {
            for pg in &extended_proxy_groups {
                if rebind_list.iter().any(|s| pg.name.starts_with(s)) {
                    keep_list.push(pg.name.clone());
                }
            }
            keep_list
        };
        pg.proxies = Some(proxies);
        extended_proxy_groups.push(pg);
    }

    for pg in &mut extended_proxy_groups {
        if let Some(pp_names) = pg.us_.take() {
            let mut new_pp_names = vec![];
            for pp_name in pp_names {
                if pp_name.starts_with('<') && pp_name.ends_with('>') {
                    let pp_name: String = pp_name.chars().skip(1).take(pp_name.len() - 2).collect();
                    new_pp_names.extend(
                        extended_proxy_providers
                            .keys()
                            .filter(|s| s.starts_with(&pp_name))
                            .cloned(),
                    );
                } else {
                    new_pp_names.push(pp_name);
                }
            }
            pg.us_ = Some(new_pp_names);
        }
    }

    tpl.insert(
        PROXY_PROVIDERS.into(),
        serde_yml::to_value(&extended_proxy_providers)?,
    );
    tpl.insert(
        PROXY_GROUPS.into(),
        serde_yml::to_value(extended_proxy_groups)?,
    );
    Ok(tpl)
}

impl BackEnd {
    /// Remove `proxy-providers` and combine their contents into one file
    ///
    /// Return combined file content
    pub fn update_profile_without_pp(
        &self,
        mut tpl: serde_yml::Mapping,
        with_proxy: bool,
    ) -> anyhow::Result<serde_yml::Mapping> {
        let Some(pps) = tpl.remove(PROXY_PROVIDERS) else {
            // if there is not proxy-providers in file, just return
            return Ok(tpl);
        };
        let pps: HashMap<String, PPitem> = serde_yml::from_value(pps)?;
        // pp_name with proxies
        let mut pp_proxies: HashMap<String, Vec<serde_yml::Value>> = HashMap::new();
        for (pp_name, pp) in pps {
            let Some(url) = pp.url else {
                continue;
            };
            let mut loaded: serde_yml::Mapping = match self.api.mock_clash_core(url, with_proxy) {
                Ok(rdr) => serde_yml::from_reader(rdr)?,
                Err(e) => {
                    log::error!("Failed to download remote profile: {e}");
                    continue;
                }
            };

            let loaded_proxies: Vec<serde_yml::Value> = loaded
                .remove(PROXIES)
                .and_then(|v| serde_yml::from_value(v).unwrap())
                .unwrap_or_default();
            log::warn!("{:?}", loaded_proxies);
            let renamed_proxies = loaded_proxies
                .into_iter()
                .map(|mut proxy| {
                    if let Some(serde_yml::Value::String(name)) = proxy.get_mut("name") {
                        name.insert_str(0, pp_name.as_str());
                    }
                    proxy
                })
                .collect();
            pp_proxies.insert(pp_name, renamed_proxies);
        }

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

        Ok(tpl)
    }
}
