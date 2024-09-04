use super::{util::extract_domain, ClashBackend};
use crate::profile::{LocalProfile, Profile, ProfileType};
use std::fs::{create_dir_all, File};
use std::path::Path;

impl ClashBackend {
    pub fn get_current_profile(&self) -> Option<Profile> {
        self.pm.get_current()
    }

    pub fn set_current_profile(&self, pf: Profile) {
        self.pm.set_current(pf);
    }

    pub fn get_all_profiles(&self) -> Vec<Profile> {
        self.pm
            .all()
            .into_iter()
            .map(|k| self.pm.get(k).unwrap())
            .collect()
    }

    pub fn get_profile<S: AsRef<str>>(&self, name: S) -> Option<Profile> {
        self.pm.get(name)
    }

    pub fn load_local_profile<P: AsRef<Path>>(
        &self,
        pf: &Profile,
        path: P,
    ) -> anyhow::Result<LocalProfile> {
        let Profile { name, dtype } = pf;
        let content = if !path.as_ref().is_file() {
            // this means the local content does not exist
            None
        } else {
            let fp = File::open(path.as_ref())?;
            serde_yaml::from_reader(fp)?
        };
        Ok(LocalProfile {
            name: name.clone(),
            dtype: dtype.clone(),
            path: path.as_ref().to_path_buf(),
            content,
        })
    }

    pub fn update_profile(
        &self,
        profile: &LocalProfile,
        with_proxy: Option<bool>,
    ) -> anyhow::Result<Vec<String>> {
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
    /// current, if `with_proxy` is set to [None], it will be treated as false
    fn download_blob<U: Into<minreq::URL>, P: AsRef<Path>>(
        &self,
        url: U,
        path: P,
        with_proxy: Option<bool>,
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
            create_dir_all(directory)?;
        }
        let mut response = self
            .api
            .mock_clash_core(url, with_proxy.is_some_and(|b| b))?;
        let mut output_file = File::create(path)?;
        std::io::copy(&mut response, &mut output_file)?;
        Ok(())
    }

    pub fn test_profile_config(&self, path: &str, geodata_mode: bool) -> std::io::Result<String> {
        use super::ipc;
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
