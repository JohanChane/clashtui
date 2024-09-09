use super::*;
use crate::clash::ipc;
use crate::tui::tabs::profile::ProfileOp;

impl BackEnd {
    pub(super) fn handle_profile_op(&self, op: ProfileOp) -> CallBack {
        match op {
            ProfileOp::GetALL => {
                let mut composed: Vec<(String, Option<std::time::Duration>)> = self
                    .get_all_profiles()
                    .into_iter()
                    .map(|pf| {
                        (
                            pf.name.clone(),
                            self.load_local_profile(pf).ok().and_then(|lp| lp.atime()),
                        )
                    })
                    .collect();
                composed.sort();
                let (name, atime) = composed.into_iter().collect();
                CallBack::ProfileInit(name, atime)
            }
            ProfileOp::Add(name, url) => {
                self.create_profile(&name, url);
                match self.update_profile(
                    self.get_profile(name)
                        .expect("Cannot find selected profile"),
                    None,
                ) {
                    Ok(v) => CallBack::ProfileCTL(v),
                    Err(e) => CallBack::Error(e.to_string()),
                }
            }
            ProfileOp::Remove(name) => {
                if let Err(e) = self.remove_profile(
                    self.get_profile(name)
                        .expect("Cannot find selected profile"),
                ) {
                    CallBack::Error(e.to_string())
                } else {
                    CallBack::ProfileCTL(vec!["Profile is now removed".to_owned()])
                }
            }
            ProfileOp::Update(name, with_proxy) => {
                match self.update_profile(
                    self.get_profile(name)
                        .expect("Cannot find selected profile"),
                    with_proxy,
                ) {
                    Ok(v) => CallBack::ProfileCTL(v),
                    Err(e) => CallBack::Error(e.to_string()),
                }
            }
            ProfileOp::Select(name) => {
                if let Err(e) = self.select_profile(
                    self.get_profile(name)
                        .expect("Cannot find selected profile"),
                ) {
                    CallBack::Error(e.to_string())
                } else {
                    CallBack::ProfileCTL(vec!["Profile is now loaded".to_owned()])
                }
            }
            ProfileOp::Test(name, geodata_mode) => {
                let pf = self
                    .get_profile(name)
                    .expect("Cannot find selected profile");
                match self.load_local_profile(pf).and_then(|pf| {
                    self.test_profile_config(&pf.path.to_string_lossy(), geodata_mode)
                        .map_err(|e| e.into())
                }) {
                    Ok(v) => CallBack::ProfileCTL(vec![v]),
                    Err(e) => CallBack::Error(e.to_string()),
                }
            }
            ProfileOp::Preview(name) => {
                let mut lines = Vec::with_capacity(1024);
                let pf = self
                    .get_profile(name)
                    .expect("Cannot find selected profile");
                lines.push(
                    pf.dtype
                        .get_domain()
                        .unwrap_or("Imported local file".to_owned()),
                );
                lines.push(Default::default());
                match self
                    .load_local_profile(pf)
                    .and_then(|pf| match pf.content.as_ref() {
                        Some(content) => {
                            serde_yaml::to_string(content)
                                .map_err(|e| e.into())
                                .map(|content| {
                                    lines.extend(content.lines().map(|s| s.to_owned()));
                                })
                        }
                        None => {
                            lines.push("yaml file is empty. Please update it.".to_owned());
                            Ok(())
                        }
                    }) {
                    Ok(_) => CallBack::Preview(lines),
                    Err(e) => CallBack::Error(e.to_string()),
                }
            }
            ProfileOp::Edit(name) => {
                let pf = self
                    .get_profile(name)
                    .expect("Cannot find selected profile");

                match self.load_local_profile(pf).and_then(|pf| {
                    ipc::spawn(
                        "sh",
                        vec![
                            "-c",
                            self.edit_cmd
                                .replace("%s", &pf.path.to_string_lossy())
                                .as_str(),
                        ],
                    )
                    .map_err(|e| e.into())
                }) {
                    Ok(_) => CallBack::Edit,
                    Err(e) => CallBack::Error(e.to_string()),
                }
            }
        }
    }
}
