use super::*;
use crate::tui::tabs::profile::ProfileOp;

impl BackEnd {
    pub(super) fn create_profile<S: AsRef<str>, S2: AsRef<str>>(&self, name: S, url: S2) {
        self.inner.pm.insert(
            name,
            clashtui::profile::ProfileType::Url(url.as_ref().to_owned()),
        );
    }
    pub(super) fn remove_profile(&self, pf: Profile) -> anyhow::Result<()> {
        let LocalProfile { path, .. } = self.load_local_profile(&pf)?;
        Ok(std::fs::remove_file(path)?)
    }
    pub fn get_profile<S: AsRef<str>>(&self, name: S) -> Option<Profile> {
        self.inner.get_profile(name)
    }
    pub fn get_all_profiles(&self) -> Vec<Profile> {
        self.inner.get_all_profiles()
    }
    pub fn get_current_profile(&self) -> Profile {
        self.inner.get_current_profile().unwrap_or_default()
    }
    pub(super) fn load_local_profile(&self, pf: &Profile) -> anyhow::Result<LocalProfile> {
        use crate::{utils::consts, HOME_DIR};
        let path = HOME_DIR
            .get()
            .unwrap()
            .join(consts::PROFILE_PATH)
            .join(&pf.name);
        self.inner.load_local_profile(pf, path)
    }
    // TODO: plan to treat None as auto
    pub fn update_profile(
        &self,
        profile: &Profile,
        with_proxy: Option<bool>,
    ) -> anyhow::Result<Vec<String>> {
        let profile = self.load_local_profile(profile)?;
        self.inner.update_profile(&profile, with_proxy)
    }

    pub fn select_profile(&self, profile: Profile) -> anyhow::Result<()> {
        // load selected profile
        let lprofile = self.load_local_profile(&profile)?;
        // merge that into basic profile
        let mut new_profile = self.base_profile.clone();
        new_profile.merge(&lprofile)?;
        // set path to clash config file path and sync to disk
        new_profile.path = self.inner.cfg.basic.clash_cfg_pth.clone().into();
        new_profile.sync_to_disk()?;
        // after, change current profile
        self.inner.set_current_profile(profile);
        // ask clash to reload config
        self.inner
            .api
            .config_reload(&self.inner.cfg.basic.clash_cfg_pth)?;
        Ok(())
    }
}

impl BackEnd {
    pub(super) fn handle_profile_op(&self, op: ProfileOp) -> CallBack {
        match op {
            ProfileOp::GetALL => {
                let mut composed: Vec<(String, Option<std::time::Duration>)> = self
                    .get_all_profiles()
                    .into_iter()
                    .map(|p| {
                        (
                            self.load_local_profile(&p).ok().and_then(|lp| lp.atime()),
                            p.name,
                        )
                    })
                    .map(|(t, n)| (n, t))
                    .collect();
                composed.sort();
                let (name, atime) = composed.into_iter().collect();
                CallBack::ProfileInit(name, atime)
            }
            ProfileOp::Add(name, url) => {
                self.create_profile(&name, url);
                match self.update_profile(
                    &self
                        .get_profile(name)
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
                    &self
                        .get_profile(name)
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
                match self.load_local_profile(&pf).and_then(|pf| {
                    self.inner
                        .test_profile_config(&pf.path.to_string_lossy(), geodata_mode)
                        .map_err(|e| e.into())
                }) {
                    Ok(v) => CallBack::ProfileCTL(vec![v]),
                    Err(e) => CallBack::Error(e.to_string()),
                }
            }
            ProfileOp::Preview(name) => {
                let mut lines: Vec<String> = vec![];
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
                    .load_local_profile(&pf)
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
                match self.load_local_profile(&pf).and_then(|pf| {
                    clashtui::backend::ipc::spawn(&self.edit_cmd, vec![&pf.path.to_string_lossy()])
                        .map_err(|e| e.into())
                }) {
                    Ok(_) => CallBack::Edit,
                    Err(e) => CallBack::Error(e.to_string()),
                }
            }
        }
    }
}
