use super::{ProfileBackend, ProfileType, ipc};

use crate::{
    consts::MAX_SUPPORTED_TEMPLATE_VERSION,
    utils::consts::{PROFILE_PATH, TEMPLATE_PATH},
};
#[cfg(feature = "tui")]
use {super::CallBack, crate::tui::tabs::profile::TemplateOp};

mod version1;

impl ProfileBackend<'_> {
    pub fn get_all_templates(&self) -> std::io::Result<Vec<String>> {
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
    pub fn create_template(&self, path: String) -> anyhow::Result<Option<String>> {
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
    pub fn apply_template(&self, name: String) -> anyhow::Result<()> {
        let path = TEMPLATE_PATH.join(&name);
        let file = std::fs::File::open(&path)
            .inspect_err(|e| log::error!("Founding template {name}:{e}"))?;
        let map: serde_yml::Mapping = serde_yml::from_reader(file)?;
        let local_urls = self
            .pm
            .all()
            .into_iter()
            .map(|name| self.pm.get(name).unwrap())
            .flat_map(|pf| {
                if let ProfileType::Url(url) = pf.dtype {
                    Some((pf.name, url))
                } else {
                    None
                }
            })
            .collect();
        let gened = match map
            .get("clashtui_template_version")
            .and_then(|v| v.as_u64())
        {
            None | Some(1) => version1::gen_template(map, &name, local_urls)?,
            Some(_) => unimplemented!(),
        };
        let gened_name = format!("{name}.clashtui_generated");
        let path = PROFILE_PATH.join(&gened_name);
        serde_yml::to_writer(std::fs::File::create(path)?, &gened)?;
        self.pm.insert(gened_name, ProfileType::Generated(name));
        Ok(())
    }

    pub fn edit_uses(&self, name: String, profiles: Vec<String>) -> anyhow::Result<()> {
        let path = TEMPLATE_PATH.join(&name);
        let file = std::fs::File::open(&path)
            .inspect_err(|e| log::error!("Founding template {name}:{e}"))?;
        let mut map: serde_yml::Mapping = serde_yml::from_reader(file)?;

        let uses = map
            .entry("clashtui".into())
            .or_insert(serde_yml::Value::Mapping(Default::default()))
            .as_mapping_mut()
            .ok_or(anyhow::anyhow!("'clashtui' is not a map"))?
            .entry("uses".into())
            .or_insert(serde_yml::Value::Sequence(Default::default()))
            .as_sequence_mut()
            .ok_or(anyhow::anyhow!("'uses' is not a array"))?;
        uses.clear();
        uses.extend(profiles.into_iter().map(|s| s.into()));

        let file = std::fs::File::create(&path)
            .inspect_err(|e| log::error!("Founding template {name}:{e}"))?;
        serde_yml::to_writer(file, &map)?;
        Ok(())
    }

    #[cfg(feature = "tui")]
    pub(super) fn handle_template_op(&self, op: TemplateOp) -> anyhow::Result<CallBack> {
        match op {
            TemplateOp::GetALL => Ok(CallBack::TemplateInit(self.get_all_templates()?)),
            TemplateOp::Add(path) => match self.create_template(path)? {
                Some(str) => Ok(CallBack::TemplateCTL(vec![str])),
                None => Ok(CallBack::TemplateCTL(vec![
                    "clashtui_template_version not found".to_string(),
                ])),
            },
            TemplateOp::Remove(name) => {
                let path = TEMPLATE_PATH.join(&name);
                std::fs::remove_file(path)?;
                Ok(CallBack::TemplateCTL(vec![format!("{name} Removed")]))
            }
            TemplateOp::Generate(name) => {
                self.apply_template(name)?;
                Ok(CallBack::TemplateCTL(vec!["Done".to_owned()]))
            }
            TemplateOp::Preview(name) => {
                let path = TEMPLATE_PATH.join(name);
                let content = std::fs::read_to_string(path)?;
                Ok(CallBack::Preview(
                    content.lines().map(|s| s.to_owned()).collect(),
                ))
            }
            TemplateOp::Edit(name) => {
                let path = TEMPLATE_PATH.join(name);
                ipc::spawn(
                    "sh",
                    vec![
                        "-c",
                        self.edit_cmd.replace("%s", path.to_str().unwrap()).as_str(),
                    ],
                )?;
                Ok(CallBack::Edit)
            }
            TemplateOp::Uses(name, profiles) => {
                self.edit_uses(name, profiles).map(|_| CallBack::Edit)
            }
        }
    }
}

const PROXY_PROVIDERS: &str = "proxy-providers";
const PROXY_GROUPS: &str = "proxy-groups";
const PROXIES: &str = "proxies";

impl ProfileBackend<'_> {
    /// Remove `proxy-providers` and combine their contents into one file
    ///
    /// Return combined file content
    pub fn update_profile_without_pp(
        &self,
        mut tpl: serde_yml::Mapping,
        with_proxy: bool,
    ) -> anyhow::Result<serde_yml::Mapping> {
        use std::collections::HashMap;

        // why we define these again?
        // the content may change between versions
        // but only a small part will be used in this function
        #[derive(serde::Deserialize, serde::Serialize, Debug)]
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
                .and_then(|v| serde_yml::from_value(v).ok())
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
