use std::collections::HashMap;

use super::{PROXY_GROUPS, PROXY_PROVIDERS};

/// located in
/// ``` yaml
/// clashtui:
///     uses: ..
/// ```
#[derive(serde::Deserialize, serde::Serialize, Debug, Default)]
struct Config {
    /// use which profiles to generate this template,
    /// program will look up these in database,
    /// and **skip** it if not found
    ///
    /// ### Note
    /// there will be **NO** error if not even a single profile is found
    uses: Vec<String>,
}

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
    name_urls: Vec<(String, String)>,
) -> anyhow::Result<serde_yml::Mapping> {
    let local_urls: Vec<_> = if let Some(cfg) = tpl.remove("clashtui") {
        let Config { uses } = serde_yml::from_value(cfg)?;
        name_urls
            .into_iter()
            .filter(|(name, _)| uses.contains(&name))
            .map(|(_, url)| url)
            .collect()
    } else {
        Default::default()
    };

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
