use anyhow::Result;
use std::fs::File;

use crate::backend::ProfileManager;
use serde::{Deserialize, Serialize};

mod core;
#[cfg(feature = "migration_v0_2_3")]
pub mod v0_2_3;
pub use core::*;

/// under the data folder:
/// * [`BasicInfo`] basic_clash_config.yaml
/// * [`ProfileManager`] clashtui.db
/// * [`log`] clashtui.log
/// * [`ConfigFile`] config.yaml
/// * `Folder` profiles/
/// * `Folder` templates/
pub struct BuildConfig {
    pub cfg: LibConfig,
    pub edit_cmd: String,
    pub open_dir_cmd: String,
    pub timeout: Option<u64>,
    /// This is `basic_clash_config.yaml` in memory
    pub base_profile: serde_yml::Mapping,
    pub data: ProfileManager,
    pub external_controller: String,
    pub proxy_addr: String,
    pub secret: Option<String>,
    pub global_ua: Option<String>,
}
impl BuildConfig {
    pub fn init_config() -> Result<()> {
        use crate::DataDir;
        use crate::consts::{PROFILE_PATH, TEMPLATE_PATH};
        use std::fs;

        fs::create_dir_all(DataDir::get())?;

        BasicInfo::default().to_file()?;
        ConfigFile::default().to_file()?;
        ProfileManager::default().to_file()?;

        fs::create_dir(TEMPLATE_PATH.as_path())?;
        fs::create_dir(PROFILE_PATH.as_path())?;

        Ok(())
    }
    /// Load config under [crate::HOME_DIR]
    pub fn load_config() -> Result<BuildConfig> {
        let ConfigFile {
            basic,
            service,
            timeout,
            edit_cmd,
            open_dir_cmd,
            hack,
        } = ConfigFile::from_file()?;
        let data = ProfileManager::from_file()?;
        let base_profile = BasicInfo::get_raw()?;
        let (external_controller, proxy_addr, secret, global_ua) =
            BasicInfo::from_raw(base_profile.clone())?.build()?;
        let cfg = LibConfig {
            basic,
            service,
            hack,
        };

        Ok(BuildConfig {
            cfg,
            base_profile,
            data,
            edit_cmd,
            open_dir_cmd,
            timeout,
            external_controller,
            proxy_addr,
            secret,
            global_ua,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
/// Get necessary info
struct BasicInfo {
    #[serde(rename = "external-controller")]
    pub external_controller: String,
    #[serde(rename = "mixed-port")]
    pub mixed_port: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u32>,
    #[serde(rename = "socks-port")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socks_port: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    #[serde(rename = "global-ua")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_ua: Option<String>,
}
impl Default for BasicInfo {
    fn default() -> Self {
        Self {
            external_controller: "127.0.0.1:9090".to_string(),
            mixed_port: Some(7890),
            port: None,
            socks_port: None,
            secret: None,
            global_ua: None,
        }
    }
}

pub struct LibConfig {
    pub basic: Basic,
    pub service: Service,
    pub hack: Hack,
}

impl BasicInfo {
    fn to_file(&self) -> Result<()> {
        let fp = File::create(crate::consts::BASIC_PATH.as_path())?;
        Ok(serde_yml::to_writer(fp, &self)?)
    }
    // pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
    //     let fp = File::open(path)?;
    //     Ok(serde_yml::from_reader(fp)?)
    // }
    fn get_raw() -> Result<serde_yml::Mapping> {
        let fp = File::open(crate::consts::BASIC_PATH.as_path())?;
        Ok(serde_yml::from_reader(fp)?)
    }
    fn from_raw(raw: serde_yml::Mapping) -> Result<Self> {
        Ok(serde_yml::from_value(serde_yml::Value::Mapping(raw))?)
    }
}
impl BasicInfo {
    const LOCALHOST: &str = "127.0.0.1";
    fn build(self) -> Result<(String, String, Option<String>, Option<String>)> {
        use crate::consts::BASIC_PATH;
        let BasicInfo {
            mut external_controller,
            mixed_port,
            port,
            socks_port,
            secret,
            global_ua,
        } = self;

        let str = match external_controller.strip_prefix("http://") {
            Some(str) => str,
            None => external_controller.as_str(),
        };
        if let Some(after) = str.strip_prefix("0.0.0.0") {
            external_controller = format!("http://{}{}", Self::LOCALHOST, after);
        } else {
            external_controller = format!("http://{}", str);
        }

        let proxy_addr = match mixed_port.or(port) {
            Some(p) => format!("http://{}:{p}", Self::LOCALHOST),
            None => socks_port
                .map(|p| format!("socks5://{}:{p}", Self::LOCALHOST))
                .ok_or(anyhow::anyhow!(
                    "failed to load proxy_addr from {}",
                    BASIC_PATH.display()
                ))?,
        };
        Ok((external_controller, proxy_addr, secret, global_ua))
    }
}

impl ConfigFile {
    fn to_file(&self) -> Result<()> {
        let fp = File::create(crate::consts::CONFIG_PATH.as_path())?;
        serde_yml::to_writer(fp, &self)?;
        Ok(())
    }
    fn from_file() -> Result<Self> {
        let fp = File::open(crate::consts::CONFIG_PATH.as_path())?;
        Ok(serde_yml::from_reader(fp)?)
    }
}

impl ProfileManager {
    pub fn to_file(&self) -> Result<()> {
        let fp = File::create(crate::consts::DATA_PATH.as_path())?;
        serde_yml::to_writer(fp, &self)?;
        Ok(())
    }
    pub fn from_file() -> Result<Self> {
        let fp = File::open(crate::consts::DATA_PATH.as_path())?;
        Ok(serde_yml::from_reader(fp)?)
    }
}
