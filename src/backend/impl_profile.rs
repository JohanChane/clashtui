use super::{ipc, BackEnd};

use std::fs::File;

#[cfg(feature = "tui")]
use super::CallBack;
#[cfg(feature = "tui")]
use crate::tui::tabs::profile::ProfileOp;

pub(super) mod database;
mod profile;

pub use profile::{LocalProfile, Profile, ProfileType};

impl BackEnd {
    pub(super) fn create_profile<S: AsRef<str>, S2: AsRef<str>>(&self, name: S, url: S2) {
        self.pm
            .insert(name, ProfileType::Url(url.as_ref().to_owned()));
    }
    pub(super) fn remove_profile(&self, pf: Profile) -> anyhow::Result<()> {
        let pf = self.load_local_profile(pf)?;
        if let Err(e) = std::fs::remove_file(pf.path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                log::warn!("Failed to Remove profile file: {e}")
            }
        };
        self.pm.remove(pf.name);
        Ok(())
    }
    pub fn get_profile<S: AsRef<str>>(&self, name: S) -> Option<Profile> {
        self.pm.get(name)
    }
    pub fn get_all_profiles(&self) -> Vec<Profile> {
        self.pm
            .all()
            .into_iter()
            .map(|k| self.pm.get(k).unwrap())
            .collect()
    }
    pub fn get_current_profile(&self) -> Profile {
        self.pm.get_current().unwrap_or_default()
    }
    pub fn set_current_profile(&self, pf: Profile) {
        self.pm.set_current(pf);
    }

    pub(super) fn load_local_profile(&self, pf: Profile) -> anyhow::Result<LocalProfile> {
        use crate::utils::consts::PROFILE_PATH;
        let path = PROFILE_PATH.join(&pf.name);
        let mut lpf = LocalProfile::from_pf(pf, path);
        lpf.sync_from_disk()?;
        Ok(lpf)
    }

    pub fn update_profile(
        &self,
        profile: Profile,
        with_proxy: Option<bool>,
        remove_proxy_provider: bool,
    ) -> anyhow::Result<Vec<String>> {
        let LocalProfile {
            name,
            dtype,
            path,
            content,
        } = self.load_local_profile(profile)?;
        // ensure path is valid
        anyhow::ensure!(
            path.is_absolute(),
            "trying to call `download_blob` without absolute path"
        );
        let directory = path
            .parent()
            .ok_or(anyhow::anyhow!("trying to download to '/' is not allowed"))?;
        if !directory.exists() {
            std::fs::create_dir_all(directory)?;
        }
        // do update
        #[inline]
        fn update_with<F: FnOnce(&str, bool) -> crate::clash::MinreqResult>(
            url: String,
            name: String,
            path: std::path::PathBuf,
            with_proxy: bool,
            apply: F,
        ) -> String {
            let url_domain = extract_domain(&url).unwrap_or("No domain");
            match (|| -> anyhow::Result<()> {
                let mut response = apply(&url, with_proxy)?;
                let mut output_file = File::create(path)?;
                std::io::copy(&mut response, &mut output_file)?;
                Ok(())
            })() {
                // pretty output
                Ok(_) => format!("Updated: {name}({url_domain})"),
                Err(err) => {
                    log::error!("Update profile {name}:{err}");
                    format!("Not Updated: {name}({url_domain})")
                }
            }
        }
        let with_proxy = with_proxy
            .unwrap_or(self.api.check_connectivity().is_ok() && self.api.version().is_ok());
        match dtype {
            // Imported file won't update, re-import and overwrite it if necessary
            ProfileType::File => anyhow::bail!("Not upgradable"),
            // Update via the given link
            ProfileType::Url(url) => {
                let res = update_with(url, name, path, with_proxy, |url, with_proxy| {
                    self.api.mock_clash_core(url, with_proxy)
                });
                Ok(vec![res])
            }
            #[cfg(feature = "template")]
            ProfileType::Generated(template_name) => {
                // rebuild from template
                if let Err(e) = self.apply_template(template_name.clone()) {
                    anyhow::bail!("Failed to regenerate from {template_name}: {e}")
                };
                let content = if remove_proxy_provider {
                    self.update_profile_without_pp(content.unwrap_or_default(), with_proxy)?
                } else {
                    content.unwrap_or_default()
                };
                serde_yml::to_writer(std::fs::File::create(path)?, &content)?;
                Ok(vec![format!("Regenerated: {}(From {template_name})", name)])
            }
            #[cfg(not(feature = "template"))]
            ProfileType::Generated(..) => {
                anyhow::bail!("template feature not enabled in this build!")
            }
            ProfileType::Github { url, token } => {
                let res = update_with(url, name, path, with_proxy, |url, with_proxy| {
                    self.api.dl_github(url, with_proxy, token)
                });
                Ok(vec![res])
            }
            ProfileType::GitLab { url, token } => {
                let res = update_with(url, name, path, with_proxy, |url, with_proxy| {
                    self.api.dl_gitlab(url, with_proxy, token)
                });
                Ok(vec![res])
            }
        }
    }

    pub fn select_profile(&self, profile: Profile) -> anyhow::Result<()> {
        // load selected profile
        let mut lprofile = self.load_local_profile(profile.clone())?;
        // merge that into basic profile
        lprofile.merge(&self.base_profile)?;
        // set path to clash config file path and sync to disk
        lprofile.path = self.cfg.basic.clash_cfg_pth.clone().into();
        lprofile.sync_to_disk()?;
        // after, change current profile
        self.set_current_profile(profile);
        // ask clash to reload config
        self.api.config_reload(&self.cfg.basic.clash_cfg_pth)?;
        Ok(())
    }
    pub fn test_profile_config(&self, path: &str, geodata_mode: bool) -> std::io::Result<String> {
        let cmd = format!(
            "{} {} -d {} -f {} -t",
            self.cfg.basic.clash_bin_pth,
            if geodata_mode { "-m" } else { "" },
            self.cfg.basic.clash_cfg_dir,
            path,
        );
        #[cfg(target_os = "windows")]
        return ipc::exec("cmd", vec!["/C", cmd.as_str()]);
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        ipc::exec("sh", vec!["-c", cmd.as_str()])
    }
}
#[cfg(feature = "tui")]
impl BackEnd {
    pub(super) fn handle_profile_op(&self, op: ProfileOp) -> anyhow::Result<CallBack> {
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
                composed.sort_unstable();
                let (name, atime) = composed.into_iter().collect();
                Ok(CallBack::ProfileInit(name, atime))
            }
            ProfileOp::Add(name, url) => {
                self.create_profile(&name, url);
                let res = self.update_profile(self.get_profile(name).unwrap(), None, false)?;
                Ok(CallBack::ProfileCTL(res))
            }
            ProfileOp::Remove(name) => {
                self.remove_profile(self.get_profile(&name).unwrap())?;
                Ok(CallBack::ProfileCTL(vec![format!("{name} Removed")]))
            }
            ProfileOp::Update(name, with_proxy, without_pp) => {
                let res =
                    self.update_profile(self.get_profile(name).unwrap(), with_proxy, without_pp)?;
                Ok(CallBack::ProfileCTL(res))
            }
            ProfileOp::Select(name) => {
                self.select_profile(self.get_profile(name).unwrap())?;
                Ok(CallBack::ProfileCTL(vec![
                    "Profile is now loaded".to_owned()
                ]))
            }
            ProfileOp::Test(name, geodata_mode) => {
                let pf = self.get_profile(name).unwrap();
                let pf = self.load_local_profile(pf)?;
                let res = self.test_profile_config(pf.path.to_str().unwrap(), geodata_mode)?;
                Ok(CallBack::ProfileCTL(
                    res.lines().map(|s| s.to_owned()).collect(),
                ))
            }
            ProfileOp::Preview(name) => {
                let mut lines = Vec::with_capacity(512);
                let pf = self.get_profile(name).unwrap();
                let pf = self.load_local_profile(pf)?;
                lines.push(
                    pf.dtype
                        .get_domain()
                        .unwrap_or("Imported local file".to_owned()),
                );
                lines.push(Default::default());

                let content = std::fs::read_to_string(pf.path)?;
                if content.is_empty() {
                    lines.push("yaml file is empty. Please update it.".to_owned());
                } else {
                    lines.extend(content.lines().map(|s| s.to_owned()));
                }
                Ok(CallBack::Preview(lines))
            }
            ProfileOp::Edit(name) => {
                let pf = self.get_profile(name).unwrap();
                let pf = self.load_local_profile(pf)?;

                ipc::spawn(
                    "sh",
                    vec![
                        "-c",
                        self.edit_cmd
                            .replace("%s", pf.path.to_str().unwrap())
                            .as_str(),
                    ],
                )?;
                Ok(CallBack::Edit)
            }
        }
    }
}

pub fn extract_domain(url: &str) -> Option<&str> {
    if let Some(protocol_end) = url.find("://") {
        let rest = &url[(protocol_end + 3)..];
        return if let Some(path_start) = rest.find('/') {
            Some(&rest[..path_start])
        } else {
            Some(rest)
        };
    }
    None
}

/*
pub fn timestamp_to_readable(timestamp: u64) -> String {
    let duration = std::time::Duration::from_secs(timestamp);
    let datetime = std::time::UNIX_EPOCH + duration;
    let datetime: chrono::DateTime<chrono::Utc> = datetime.into();
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}
*/
