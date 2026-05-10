use super::extract_domain;
use std::{fs::File, path::PathBuf};

use crate::config::database::{Profile, ProfileType};

impl Profile {
    pub fn load_local_profile(self) -> anyhow::Result<LocalProfile> {
        use super::super::PROFILE_JSONS_PATH;
        use super::PROFILE_YAMLS_PATH;
        let path = if matches!(self.dtype, ProfileType::Singbox)
            || crate::config::CONFIG.core_type() == crate::config::CoreType::Singbox
        {
            PROFILE_JSONS_PATH.join(format!("{}.json", &self.name))
        } else {
            PROFILE_YAMLS_PATH.join(format!("{}.yaml", &self.name))
        };
        let mut lpf = LocalProfile::from_pf(self, path);
        lpf.sync_from_disk()?;
        Ok(lpf)
    }
}

#[derive(Clone)]
pub struct LocalProfile {
    pub name: String,
    pub dtype: ProfileType,
    pub path: PathBuf,
    pub content: Option<serde_yml::Mapping>,
}
impl Default for LocalProfile {
    fn default() -> Self {
        Self {
            name: "base".into(),
            dtype: ProfileType::File,
            path: Default::default(),
            content: Default::default(),
        }
    }
}

impl LocalProfile {
    /// Returns the atime of this [`LocalProfile`].
    ///
    /// Errors are ignored and return will be replaced with [None]
    pub fn atime(&self) -> Option<core::time::Duration> {
        pub fn get_modify_time<P>(file_path: P) -> std::io::Result<std::time::SystemTime>
        where
            P: AsRef<std::path::Path>,
        {
            let file = std::fs::metadata(file_path)?;
            if file.is_file() {
                file.modified()
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Not a file",
                ))
            }
        }
        let now = std::time::SystemTime::now();
        get_modify_time(&self.path)
            .ok()
            .and_then(|file| now.duration_since(file).ok())
    }
    /// merge `core_override_config` to `self::content`,
    /// all top-level keys in `core_override_config` overwrite the profile's values
    ///
    /// Note: need to call [`LocalProfile::sync_from_disk`] before call this
    pub fn merge(&mut self, core_override_config: &serde_yml::Mapping) -> anyhow::Result<()> {
        if self.content.is_none() || core_override_config.is_empty() {
            log::warn!("skip merge: one of the input content is none");
            return Ok(());
        }
        let map = self.content.as_mut().unwrap();
        for (key, value) in core_override_config.iter() {
            map.insert(key.clone(), value.clone());
        }
        Ok(())
    }
    /// sync the content to disk by [`LocalProfile::path`]
    pub fn sync_to_disk(self) -> anyhow::Result<()> {
        let LocalProfile { path, content, .. } = self;
        let fp = File::create(path)
            .map_err(|e| anyhow::anyhow!("Failed to write clash config file: {e}"))?;
        Ok(serde_yml::to_writer(fp, &content)?)
    }
    pub fn from_pf(pf: Profile, path: std::path::PathBuf) -> Self {
        let Profile { name, dtype, .. } = pf;
        Self {
            name,
            dtype,
            path,
            content: None,
        }
    }
    /// sync the content from disk by [`LocalProfile::path`]
    pub fn sync_from_disk(&mut self) -> anyhow::Result<()> {
        if self.path.is_file() {
            let fp = File::open(&self.path)?;
            self.content = Some(serde_yml::from_reader(fp)?);
        }
        Ok(())
    }
}

impl ProfileType {
    pub fn get_domain(&self) -> Option<String> {
        match self {
            ProfileType::File | ProfileType::Singbox | ProfileType::Template { .. } => None,
            ProfileType::Url(url) => extract_domain(url).map(|s| s.to_owned()),
        }
    }
}
