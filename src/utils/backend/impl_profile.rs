use super::*;
use crate::clash::{ipc, profile::ProfileType, util::extract_domain};
use std::fs::File;

impl BackEnd {
    pub(super) fn create_profile<S: AsRef<str>, S2: AsRef<str>>(&self, name: S, url: S2) {
        self.pm
            .insert(name, ProfileType::Url(url.as_ref().to_owned()));
    }
    pub(super) fn remove_profile(&self, pf: Profile) -> anyhow::Result<()> {
        let name = pf.name.clone();
        let LocalProfile { path, .. } = self.load_local_profile(pf)?;
        std::fs::remove_file(path)?;
        self.pm.remove(name);
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
        use crate::{utils::consts, HOME_DIR};
        let path = HOME_DIR.join(consts::PROFILE_PATH).join(&pf.name);
        let mut lpf = LocalProfile::from_pf(pf, path);
        lpf.sync_from_disk()?;
        Ok(lpf)
    }

    pub fn update_profile(
        &self,
        profile: Profile,
        with_proxy: Option<bool>,
    ) -> anyhow::Result<Vec<String>> {
        let profile = self.load_local_profile(profile)?;
        let with_proxy = with_proxy
            .unwrap_or(self.api.check_connectivity().is_ok() && self.api.version().is_ok());
        if profile.dtype.is_upgradable() {
            // store (name,url) to be downloaded
            let mut work_vec: Vec<(String, String)> = Vec::with_capacity(2);
            match &profile.dtype {
                // Imported file won't update, overwrite it if necessary
                ProfileType::File => unreachable!(),
                // Update via the given link
                ProfileType::Url(url) => {
                    work_vec.push((url.clone(), profile.path.to_string_lossy().to_string()))
                }
                #[cfg(feature = "template")]
                ProfileType::Generated(_template_name) => {
                    // rebuild from template
                    todo!()
                }
            }
            Ok(work_vec
                .into_iter()
                .map(|(url, path)| {
                    // pretty output
                    let url_domain = extract_domain(url.as_str()).unwrap_or("No domain");
                    let profile_name = &profile.name;
                    match self.download_blob(&url, path, with_proxy) {
                        Ok(_) => format!("Updated: {profile_name}({url_domain})"),
                        Err(err) => {
                            log::error!("Update profile {profile_name}:{err}");
                            format!("Not Updated: {profile_name}({url_domain})")
                        }
                    }
                })
                .collect::<Vec<String>>())
        } else {
            anyhow::bail!("Not upgradable");
        }
    }

    fn download_blob<U: Into<minreq::URL>, P: AsRef<std::path::Path>>(
        &self,
        url: U,
        path: P,
        with_proxy: bool,
    ) -> anyhow::Result<()> {
        assert!(
            path.as_ref().is_absolute(),
            "trying to call `download_blob` without absolute path"
        );
        let directory = path
            .as_ref()
            .parent()
            .ok_or(anyhow::anyhow!("trying to download to '/' is not allowed"))?;
        if !directory.exists() {
            std::fs::create_dir_all(directory)?;
        }
        let mut response = self.api.mock_clash_core(url, with_proxy)?;
        let mut output_file = File::create(path)?;
        std::io::copy(&mut response, &mut output_file)?;
        Ok(())
    }

    pub fn select_profile(&self, profile: Profile) -> anyhow::Result<()> {
        // load selected profile
        let lprofile = self.load_local_profile(profile.clone())?;
        // merge that into basic profile
        let mut new_profile = self.base_profile.clone();
        new_profile.merge(&lprofile)?;
        // set path to clash config file path and sync to disk
        new_profile.path = self.cfg.basic.clash_cfg_pth.clone().into();
        new_profile.sync_to_disk()?;
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
