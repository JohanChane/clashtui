use anyhow::Result;
use std::fs::File;
use std::path::Path;

use crate::clash::{
    config::{Basic, Service},
    profile::map::ProfileDataBase,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ConfigFile {
    pub basic: Basic,
    pub service: Service,
    pub timeout: Option<u64>,
    pub edit_cmd: String,
}
impl ConfigFile {
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

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct DataFile {
    pub profiles: ProfileDataBase,
    pub current_profile: String,
}
impl DataFile {
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
    DataFile::default().to_file(data_path)?;

    fs::create_dir(template_dir)?;
    fs::create_dir(profile_dir)?;

    // fs::write(config_dir.join(BASIC_FILE), DEFAULT_BASIC_CLASH_CFG_CONTENT)?;
    Ok(())
}

pub fn load_config(config_dir: &Path) -> Result<BuildConfig> {
    use crate::consts::{BASIC_FILE, CONFIG_FILE, DATA_FILE};

    let basic_path = config_dir.join(BASIC_FILE);
    let config_path = config_dir.join(CONFIG_FILE);
    let data_path = config_dir.join(DATA_FILE);

    let cfg = ConfigFile::from_file(config_path)?;
    let data = DataFile::from_file(data_path)?;
    let raw = BasicInfo::get_raw(basic_path)?;
    let base = BasicInfo::from_raw(raw.clone())?;

    Ok(BuildConfig {
        cfg,
        basic: base,
        base_raw: raw,
        data,
    })
}

pub struct BuildConfig {
    pub cfg: ConfigFile,
    pub basic: BasicInfo,
    pub base_raw: serde_yml::Mapping,
    pub data: DataFile,
}

#[derive(Debug, Serialize, Deserialize)]
/// Get necessary info
pub struct BasicInfo {
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
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let fp = File::create(path)?;
        Ok(serde_yml::to_writer(fp, &self)?)
    }
    // pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
    //     let fp = File::open(path)?;
    //     Ok(serde_yml::from_reader(fp)?)
    // }
    pub fn get_raw<P: AsRef<Path>>(path: P) -> Result<serde_yml::Mapping> {
        let fp = File::open(path)?;
        Ok(serde_yml::from_reader(fp)?)
    }
    pub fn from_raw(raw: serde_yml::Mapping) -> Result<Self> {
        Ok(serde_yml::from_value(serde_yml::Value::Mapping(raw))?)
    }
}
impl BasicInfo {
    pub fn build(self) -> Result<(String, String, Option<String>, Option<String>)> {
        use crate::consts::{BASIC_FILE, LOCALHOST};
        let BasicInfo {
            mut external_controller,
            mixed_port,
            port,
            socks_port,
            secret,
            global_ua,
        } = self;

        if external_controller.starts_with("0.0.0.0") {
            external_controller = format!(
                "127.0.0.1{}",
                external_controller.strip_prefix("0.0.0.0").unwrap()
            );
        }
        external_controller = format!("http://{external_controller}");
        let proxy_addr = match mixed_port
            .or(port)
            .map(|p| format!("http://{LOCALHOST}:{p}"))
        {
            Some(s) => s,
            None => socks_port
                .map(|p| format!("socks5://{LOCALHOST}:{p}"))
                .ok_or(anyhow::anyhow!(
                    "failed to load proxy_addr from {BASIC_FILE}"
                ))?,
        };
        Ok((external_controller, proxy_addr, secret, global_ua))
    }
}
