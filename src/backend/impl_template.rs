use super::{ipc, BackEnd, ProfileType};

#[cfg(feature = "tui")]
use super::CallBack;
#[cfg(feature = "tui")]
use crate::tui::tabs::profile::TemplateOp;
use crate::{
    utils::consts::{PROFILE_PATH, TEMPLATE_PATH},
    HOME_DIR,
};

mod version1;

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
            None => {
                // file is opened, so file_name should exist
                let name_maybe_with_ext = path.file_name().unwrap().to_str().unwrap();
                let name = name_maybe_with_ext
                    // remove the last one only
                    // e.g. this.tar.gz => this.tar
                    .rsplit_once('.')
                    .map(|(v, _)| v)
                    .unwrap_or(name_maybe_with_ext);
                std::fs::copy(&path, HOME_DIR.join(TEMPLATE_PATH).join(name))?;
                Ok(None)
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
        let local_urls = self
            .pm
            .all()
            .into_iter()
            .map(|name| self.pm.get(name).unwrap())
            .flat_map(|pf| {
                if let ProfileType::Url(url) = pf.dtype {
                    Some(url)
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
        let path = HOME_DIR.join(PROFILE_PATH).join(&gened_name);
        serde_yml::to_writer(std::fs::File::create(path)?, &gened)?;
        self.pm.insert(gened_name, ProfileType::Generated(name));
        Ok(())
    }
}

#[cfg(feature = "tui")]
impl BackEnd {
    pub(super) fn handle_template_op(&self, op: TemplateOp) -> anyhow::Result<CallBack> {
        match op {
            TemplateOp::GetALL => Ok(CallBack::TemplateInit(self.get_all_templates()?)),
            TemplateOp::Add(path) => match self.create_template(path)? {
                Some(str) => Ok(CallBack::TemplateCTL(vec![str])),
                None => Ok(CallBack::TemplateCTL(vec![
                    "clashtui_template_version not found".to_string()
                ])),
            },
            TemplateOp::Remove(name) => {
                let path = HOME_DIR.join(TEMPLATE_PATH).join(&name);
                std::fs::remove_file(path)?;
                Ok(CallBack::TemplateCTL(vec![format!("{name} Removed")]))
            }
            TemplateOp::Generate(name) => {
                self.apply_template(name)?;
                Ok(CallBack::TemplateCTL(vec!["Done".to_owned()]))
            }
            TemplateOp::Preview(name) => {
                let path = HOME_DIR.join(TEMPLATE_PATH).join(name);
                let content = std::fs::read_to_string(path)?;
                Ok(CallBack::Preview(
                    content.lines().map(|s| s.to_owned()).collect(),
                ))
            }
            TemplateOp::Edit(name) => {
                let path = HOME_DIR.join(TEMPLATE_PATH).join(name);
                ipc::spawn(
                    "sh",
                    vec![
                        "-c",
                        self.edit_cmd.replace("%s", path.to_str().unwrap()).as_str(),
                    ],
                )?;
                Ok(CallBack::Edit)
            }
        }
    }
}
