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
            None => {
                todo!("fallback")
            }
            Some(ver) if ver <= 1 => {
                // file is opened, so file_name should exist
                let name_maybe_with_ext = path.file_name().unwrap().to_str().unwrap();
                let name = name_maybe_with_ext
                    // remove the last one only
                    // e.g. this.tar.gz => this.tar
                    .rsplit_once('.')
                    .unwrap_or((name_maybe_with_ext, ""))
                    .0;
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
            None => {
                todo!("fallback")
            }
            Some(1) => {
                let gened = template_ver1(map)?;
                let gened_name = format!("{name}.clashtui_generated");
                let path = HOME_DIR.join(PROFILE_PATH).join(&gened_name);
                serde_yml::to_writer(std::fs::File::create(path)?, &gened)?;
                self.pm.insert(gened_name, ProfileType::Generated(name));
            }
            Some(_) => unimplemented!(),
        }
        fn template_ver1(tpl: serde_yml::Mapping) -> anyhow::Result<serde_yml::Mapping> {
            Ok(Default::default())
        }
        Ok(())
    }
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
