use profile::ProfileType;

use super::*;
#[cfg(feature = "tui")]
use crate::tui::tabs::profile::TemplateOp;
use crate::{
    utils::consts::{PROFILE_PATH, TEMPLATE_PATH},
    HOME_DIR,
};

impl BackEnd {
    pub fn get_all_templates(&self) -> std::io::Result<Vec<String>> {
        let dir_path = HOME_DIR.join(TEMPLATE_PATH);
        Ok(std::fs::read_dir(dir_path)?
            .collect::<std::io::Result<Vec<std::fs::DirEntry>>>()?
            .into_iter()
            .map(|p| {
                p.file_name()
                    .into_string()
                    .unwrap_or("Containing non UTF-8 char".to_owned())
            })
            .collect())
    }
    pub fn create_template(&self, path: String) -> anyhow::Result<Option<String>> {
        let path = std::path::PathBuf::from(path);
        let file = std::fs::File::open(&path)?;
        let map: serde_yml::Mapping = serde_yml::from_reader(file)?;
        match map
            .get("clashtui_template_version")
            .and_then(|v| v.as_u64())
        {
            // regard as version 1
            None => {
                let ver = 1;
                // file is opened, so file_name should exist
                let name_maybe_with_ext = path.file_name().unwrap().to_str().unwrap();
                let name = name_maybe_with_ext
                    // remove the last one only
                    // e.g. this.tar.gz => this.tar
                    .rsplit_once('.')
                    .map(|(v, _)| v)
                    .unwrap_or(name_maybe_with_ext);
                std::fs::copy(&path, HOME_DIR.join(TEMPLATE_PATH).join(name))?;
                Ok(Some(format!(
                    "Name:{} Added\nClashtui Template Version {}",
                    // path from a String, should be UTF-8
                    name,
                    ver
                )))
            }
            Some(ver) if ver <= 1 => {
                // file is opened, so file_name should exist
                let name_maybe_with_ext = path.file_name().unwrap().to_str().unwrap();
                let name = name_maybe_with_ext
                    // remove the last one only
                    // e.g. this.tar.gz => this.tar
                    .rsplit_once('.')
                    .map(|(v, _)| v)
                    .unwrap_or(name_maybe_with_ext);
                std::fs::copy(&path, HOME_DIR.join(TEMPLATE_PATH).join(name))?;
                Ok(Some(format!(
                    "Name:{} Added\nClashtui Template Version {}",
                    // path from a String, should be UTF-8
                    name,
                    ver
                )))
            }
            Some(_) => unimplemented!(),
        }
    }
    pub fn apply_template(&self, name: String) -> anyhow::Result<()> {
        let path = HOME_DIR.join(TEMPLATE_PATH).join(&name);
        let file = std::fs::File::open(&path)
            .inspect_err(|e| log::debug!("Founding template {name}:{e}"))?;
        let map: serde_yml::Mapping = serde_yml::from_reader(file)?;
        match map
            .get("clashtui_template_version")
            .and_then(|v| v.as_u64())
        {
            None | Some(1) => {
                let gened = template_ver1(map, &name, vec![])?;
                let gened_name = format!("{name}.clashtui_generated");
                let path = HOME_DIR.join(PROFILE_PATH).join(&gened_name);
                serde_yml::to_writer(std::fs::File::create(path)?, &gened)?;
                self.pm.insert(gened_name, ProfileType::Generated(name));
            }
            Some(_) => unimplemented!(),
        }
        Ok(())
    }
}

fn template_ver1(
    mut tpl: serde_yml::Mapping,
    tpl_name: &str,
    local_urls: Vec<String>,
) -> anyhow::Result<serde_yml::Mapping> {
    use std::collections::HashMap;
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
        typ_: String,
        /// not cared, just keep this
        #[serde(flatten)]
        others: serde_yml::Value,
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
        typ_: String,
    }
    // proxy-groups
    let pgs = tpl
        .remove("proxy-groups")
        .ok_or(anyhow::anyhow!("proxy-groups not found"))?;
    let pgs: Vec<PGitem> = serde_yml::from_value(pgs)?;
    // proxy-providers
    let pps = tpl
        .remove("proxy-providers")
        .ok_or(anyhow::anyhow!("proxy-providers not found"))?;
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
        "proxy-providers".into(),
        serde_yml::to_value(&extended_proxy_providers)?,
    );
    tpl.insert(
        "proxy-groups".into(),
        serde_yml::to_value(extended_proxy_groups)?,
    );
    Ok(tpl)
}

#[cfg(feature = "tui")]
impl BackEnd {
    pub(super) fn handle_template_op(&self, op: TemplateOp) -> CallBack {
        match op {
            TemplateOp::GetALL => match self.get_all_templates() {
                Ok(v) => CallBack::TemplateInit(v),
                Err(e) => CallBack::Error(e.to_string()),
            },
            TemplateOp::Add(path) => match self.create_template(path) {
                Ok(Some(str)) => CallBack::TemplateCTL(vec![str]),
                Ok(None) => {
                    CallBack::TemplateCTL(vec!["Not a valid clashtui template".to_string()])
                }
                Err(e) => CallBack::Error(e.to_string()),
            },
            TemplateOp::Remove(name) => {
                let path = HOME_DIR.join(TEMPLATE_PATH).join(&name);
                match std::fs::remove_file(path) {
                    Ok(()) => CallBack::TemplateCTL(vec![format!("{name} Removed")]),
                    Err(e) => CallBack::Error(e.to_string()),
                }
            }
            TemplateOp::Generate(name) => match self.apply_template(name) {
                Ok(()) => CallBack::TemplateCTL(vec![]),
                Err(e) => CallBack::Error(e.to_string()),
            },
            TemplateOp::Preview(name) => {
                let path = HOME_DIR.join(TEMPLATE_PATH).join(name);
                match std::fs::read_to_string(path) {
                    Ok(content) => {
                        CallBack::Preview(content.lines().map(|s| s.to_owned()).collect())
                    }
                    Err(e) => CallBack::Error(e.to_string()),
                }
            }
            TemplateOp::Edit(name) => {
                let path = HOME_DIR.join(TEMPLATE_PATH).join(name);
                match ipc::spawn(
                    "sh",
                    vec![
                        "-c",
                        self.edit_cmd.replace("%s", path.to_str().unwrap()).as_str(),
                    ],
                ) {
                    Ok(()) => CallBack::Edit,
                    Err(e) => CallBack::Error(e.to_string()),
                }
            }
        }
    }
}
