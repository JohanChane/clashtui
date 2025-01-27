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
                let gened = template_ver1(map, &name)?;
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
) -> anyhow::Result<serde_yml::Mapping> {
    macro_rules! expand {
        ($pats:pat, $exprs:expr) => {
            let $pats = $exprs else {
                anyhow::bail!(
                    "Failed to find {} in {}",
                    stringify!($pats),
                    stringify!($exprs)
                )
            };
        };
    }
    #[derive(Hash, PartialEq, Eq)]
    enum PGproxies {
        FullName(String),
        Prefix(String),
    }
    let local_urls = vec!["1".to_owned()];
    // proxy-providers with proxy-groups
    // relationship between proxy-providers and proxy-groups
    let mut relation = {
        let mut relation: std::collections::HashMap<serde_yml::Value, Vec<serde_yml::Value>> =
            std::collections::HashMap::new();
        expand!(
            Some(serde_yml::Value::Sequence(proxy_groups)),
            tpl.remove("proxy-groups")
        );
        //  - name: "Sl"
        //    tpl_param:
        //      providers: ["pvd"]
        //    type: select
        for value in proxy_groups {
            if value.get("tpl_param").is_none() {
                relation
                    .entry(serde_yml::Value::Null)
                    .or_default()
                    .push(value);
                continue;
            }
            expand!(serde_yml::Value::Mapping(mut value), value);
            expand!(
                Some(serde_yml::Value::Mapping(mut param)),
                value.remove("tpl_param")
            );
            expand!(
                Some(serde_yml::Value::Sequence(pvds)),
                param.remove("providers")
            );
            for pvd in pvds {
                relation
                    .entry(pvd)
                    .or_default()
                    .push(serde_yml::Value::Mapping(value.clone()));
            }
        }
        relation
    };
    // proxy-groups with proxy-groups's name
    let mut relation2: std::collections::HashMap<
        serde_yml::Value,
        std::collections::HashSet<PGproxies>,
    > = std::collections::HashMap::new();
    // relationship between proxy-groups
    {
        // - name: "Entry"
        //   type: select
        //   proxies:
        //     - Common
        //     - <At>
        //     - <Sl>
        for (_key, value) in &relation {
            for value in value {
                for va in value
                    .get("proxies")
                    .and_then(|p| p.as_sequence())
                    .iter()
                    .flat_map(|v| v.iter())
                    .map(|i| i.as_str().unwrap().to_owned())
                {
                    if va.starts_with('<') && va.ends_with('>') {
                        let trimed_str = va.trim_start_matches('<').trim_end_matches('>');
                        relation2
                            .entry(value.clone())
                            .or_default()
                            .insert(PGproxies::Prefix(trimed_str.to_owned()));
                    } else {
                        relation2
                            .entry(value.clone())
                            .or_default()
                            .insert(PGproxies::FullName(va));
                    }
                }
            }
        }
    }
    // proxy-providers
    {
        expand!(
            Some(serde_yml::Value::Mapping(pp)),
            tpl.remove("proxy-providers")
        );
        let mut extended_proxy_providers = serde_yml::Mapping::new();
        let mut extended_proxy_groups = serde_yml::Sequence::new();
        // proxy_provider_name:
        //   tpl_param:
        //   type: http
        for (key, value) in pp {
            if value.get("tpl_param").is_none() {
                continue;
            }
            // asserts
            expand!(serde_yml::Value::Mapping(mut content), value);
            expand!(serde_yml::Value::String(name), key);
            expand!(
                Some(pgs),
                relation.remove(&serde_yml::Value::String(name.clone()))
            );
            // remove marker
            content.remove("tpl_param");

            for (i, url) in local_urls.iter().enumerate() {
                use serde_yml::Value::String;
                let mut spp = content.clone();
                let proxy_provider_name = format!("{name}{i}");
                spp.insert(String("url".to_string()), String(url.clone()));
                spp.insert(
                    String("path".to_string()),
                    String(format!(
                        "proxy-providers/tpl/{tpl_name}/{proxy_provider_name}.yaml"
                    )),
                );
                extended_proxy_providers.insert(
                    String(proxy_provider_name.clone()),
                    serde_yml::Value::Mapping(spp),
                );
                // proxy-groups
                for pg in pgs.clone() {
                    expand!(serde_yml::Value::Mapping(mut pg), pg);
                    expand!(Some(serde_yml::Value::String(pg_name)), pg.remove("name"));
                    pg.insert(
                        "name".into(),
                        format!("{pg_name}-{proxy_provider_name}").into(),
                    );
                    pg.insert("use".into(), vec![proxy_provider_name.clone()].into());
                    extended_proxy_groups.push(serde_yml::Value::Mapping(pg));
                }
            }
        }
        expand!(Some(pgs), relation.remove(&serde_yml::Value::Null));
        extended_proxy_groups.extend(pgs);
        for pg in &mut extended_proxy_groups {
            if let Some(providers) = pg.get("use") {
                let mut new_providers = Vec::new();
                for p in providers.as_sequence().unwrap() {
                    let p_str = p.as_str().unwrap();
                    if p_str.starts_with('<') && p_str.ends_with('>') {
                        let trimmed_p_str = p_str.trim_start_matches('<').trim_end_matches('>');
                        new_providers.extend(
                            extended_proxy_providers
                                .iter()
                                .map(|(k, _)| k.as_str().unwrap())
                                .filter(|n| n.starts_with(trimmed_p_str))
                                .map(|s| s.to_owned()),
                        );
                    } else {
                        new_providers.push(p_str.to_string());
                    }
                }
                pg["use"] = serde_yml::Value::Sequence(
                    new_providers
                        .into_iter()
                        .map(serde_yml::Value::String)
                        .collect(),
                );
            }
        }
        for (pg_, name) in relation2 {
            let mut name_vec: Vec<serde_yml::Value> = vec![];
            for pg in &extended_proxy_groups {
                let pg_name = pg.get("name").unwrap().as_str().unwrap();
                if name.iter().any(|name| match name {
                    PGproxies::Prefix(name) => pg_name.starts_with(name),
                    PGproxies::FullName(name) => pg_name == name,
                }) {
                    name_vec.push(pg_name.into());
                }
            }
            for pg in &mut extended_proxy_groups {
                let pg_name = pg.get("name").unwrap();
                if pg_.get("name").unwrap() == pg_name {
                    pg["proxies"] = name_vec.into();
                    break;
                }
            }
        }
        tpl.insert(
            "proxy-providers".into(),
            serde_yml::Value::Mapping(extended_proxy_providers),
        );
        tpl.insert(
            "proxy-groups".into(),
            serde_yml::Value::Sequence(extended_proxy_groups),
        );
    }
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
