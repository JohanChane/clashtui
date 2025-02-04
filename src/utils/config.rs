use anyhow::Result;
use std::fs::File;
use std::path::Path;

use crate::backend::ProfileManager;
use serde::{Deserialize, Serialize};

#[cfg(feature = "migration")]
pub mod v0_2_3;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Basic {
    #[serde(rename = "clash_config_dir")]
    pub clash_cfg_dir: String,
    #[serde(rename = "clash_bin_path")]
    pub clash_bin_pth: String,
    #[serde(rename = "clash_config_path")]
    pub clash_cfg_pth: String,
}
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Service {
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    pub clash_srv_nam: String,
    #[cfg(target_os = "linux")]
    pub is_user: bool,
}

pub struct LibConfig {
    pub basic: Basic,
    pub service: Service,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
struct ConfigFile {
    pub basic: Basic,
    pub service: Service,
    pub timeout: Option<u64>,
    pub edit_cmd: String,
}
impl ConfigFile {
    fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let fp = File::create(path)?;
        serde_yml::to_writer(fp, &self)?;
        Ok(())
    }
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let fp = File::open(path)?;
        Ok(serde_yml::from_reader(fp)?)
    }
}

impl ProfileManager {
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let fp = File::create(path)?;
        serde_yml::to_writer(fp, &self)?;
        Ok(())
    }
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let fp = File::open(path)?;
        Ok(serde_yml::from_reader(fp)?)
    }
}

pub struct BuildConfig {
    pub cfg: LibConfig,
    pub edit_cmd: String,
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
    pub fn init_config(config_dir: &Path) -> Result<()> {
        use crate::consts::{BASIC_FILE, CONFIG_FILE, DATA_FILE, PROFILE_PATH, TEMPLATE_PATH};
        use std::fs;

        let template_dir = config_dir.join(TEMPLATE_PATH);
        let profile_dir = config_dir.join(PROFILE_PATH);
        let basic_path = config_dir.join(BASIC_FILE);
        let config_path = config_dir.join(CONFIG_FILE);
        let data_path = config_dir.join(DATA_FILE);

        fs::create_dir_all(config_dir)?;

        BasicInfo::default().to_file(basic_path)?;
        ConfigFile::default().to_file(config_path)?;
        ProfileManager::default().to_file(data_path)?;

        fs::create_dir(template_dir)?;
        fs::create_dir(profile_dir)?;

        Ok(())
    }

    pub fn load_config(config_dir: &Path) -> Result<BuildConfig> {
        use crate::consts::{BASIC_FILE, CONFIG_FILE, DATA_FILE};

        let basic_path = config_dir.join(BASIC_FILE);
        let config_path = config_dir.join(CONFIG_FILE);
        let data_path = config_dir.join(DATA_FILE);

        let ConfigFile {
            basic,
            service,
            timeout,
            edit_cmd,
        } = ConfigFile::from_file(config_path)?;
        let data = ProfileManager::from_file(data_path)?;
        let base_profile = BasicInfo::get_raw(basic_path)?;
        let (external_controller, proxy_addr, secret, global_ua) =
            BasicInfo::from_raw(base_profile.clone())?.build()?;
        let cfg = LibConfig { basic, service };

        Ok(BuildConfig {
            cfg,
            base_profile,
            data,
            edit_cmd,
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
impl BasicInfo {
    fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let fp = File::create(path)?;
        Ok(serde_yml::to_writer(fp, &self)?)
    }
    // pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
    //     let fp = File::open(path)?;
    //     Ok(serde_yml::from_reader(fp)?)
    // }
    fn get_raw<P: AsRef<Path>>(path: P) -> Result<serde_yml::Mapping> {
        let fp = File::open(path)?;
        Ok(serde_yml::from_reader(fp)?)
    }
    fn from_raw(raw: serde_yml::Mapping) -> Result<Self> {
        Ok(serde_yml::from_value(serde_yml::Value::Mapping(raw))?)
    }
}
impl BasicInfo {
    const LOCALHOST: &str = "127.0.0.1";
    fn build(self) -> Result<(String, String, Option<String>, Option<String>)> {
        use crate::consts::BASIC_FILE;
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
                    "failed to load proxy_addr from {BASIC_FILE}"
                ))?,
        };
        Ok((external_controller, proxy_addr, secret, global_ua))
    }
}
