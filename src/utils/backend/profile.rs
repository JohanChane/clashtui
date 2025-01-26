use std::{fs::File, path::PathBuf};

pub mod map;

pub use map::ProfileType;

#[derive(Clone)]
pub struct Profile {
    pub name: String,
    pub dtype: ProfileType,
}
impl Default for Profile {
    fn default() -> Self {
        Self {
            name: "Unknown".into(),
            dtype: ProfileType::File,
        }
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
    const FILTER: [&str; 6] = [
        "proxy-groups",
        "proxy-providers",
        "proxies",
        "sub-rules",
        "rules",
        "rule-providers",
    ];
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
    /// merge `base` into [`LocalProfile::content`],
    /// using [`FILTER`](Self::FILTER)
    ///
    /// Note: need to call [`LocalProfile::sync_from_disk`] before call this
    pub fn merge(&mut self, base: &LocalProfile) -> anyhow::Result<()> {
        if self.content.is_none() || base.content.is_none() {
            anyhow::bail!("failed to merge: one of the input content is none");
        }

        Self::FILTER
            .into_iter()
            .filter(|s| {
                base.content
                    .as_ref()
                    .expect("need to call sync from disk")
                    .contains_key(s)
            })
            .map(|key| (key, base.content.as_ref().unwrap().get(key).unwrap()))
            .for_each(|(k, v)| {
                self.content.as_mut().unwrap().insert(k.into(), v.clone());
            });
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
        let Profile { name, dtype } = pf;
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
            self.content = serde_yml::from_reader(fp)?;
        }
        Ok(())
    }
}
