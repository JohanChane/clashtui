use api::ClashUtil;
use serde::{Deserialize, Serialize};
use std::fs::File;

use crate::const_err::ERR_PATH_UTF_8;

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct ProfileMap(core::cell::RefCell<std::collections::HashMap<String, ProfileType>>);
impl ProfileMap {
    pub fn insert<S: AsRef<str>>(&self, name: S, path: ProfileType) -> Option<ProfileType> {
        self.0.borrow_mut().insert(name.as_ref().to_string(), path)
    }
    pub fn get<S: AsRef<str>>(&self, name: S) -> Option<ProfileType> {
        self.0.borrow().get(name.as_ref()).cloned()
    }
    pub fn all(&self) -> Vec<String> {
        self.0.borrow().keys().cloned().collect()
    }
    pub fn remove<S: AsRef<str>>(&self, name: S) -> Option<ProfileType> {
        self.0.borrow_mut().remove(name.as_ref())
    }
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ProfileType {
    /// local import
    File,
    /// download url
    Url(String),
    /// generated by template
    Generated(String),
}
impl ProfileType {
    pub fn is_null(&self) -> bool {
        matches!(self, ProfileType::File)
    }
    pub fn into_inner(self) -> Option<String> {
        match self {
            ProfileType::File => None,
            ProfileType::Url(s) => Some(s),
            ProfileType::Generated(s) => Some(s),
        }
    }
}
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct Basic {
    clash_cfg_dir: String,
    clash_bin_pth: String,
    clash_cfg_pth: String,
    timeout: Option<u64>,
}
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct Extra {
    edit_cmd: String,
    open_dir_cmd: String,
}
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct Service {
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    clash_srv_nam: String,
    #[cfg(target_os = "linux")]
    is_user: bool,
}
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct ConfigFile {
    basic: Basic,
    service: Service,
    extra: Extra,
}
impl ConfigFile {
    pub fn from_file(config_path: &str) -> anyhow::Result<Self> {
        let f = File::open(config_path)?;
        Ok(serde_yaml::from_reader(f)?)
    }

    pub fn to_file(&self, config_path: &str) -> anyhow::Result<()> {
        let f = File::create(config_path).unwrap();
        Ok(serde_yaml::to_writer(f, self)?)
    }
}
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct DataFile {
    current_profile: core::cell::RefCell<String>,
    profiles: ProfileMap,
}
impl DataFile {
    pub fn from_file(config_path: &str) -> anyhow::Result<Self> {
        let f = File::open(config_path)?;
        Ok(serde_yaml::from_reader(f)?)
    }

    pub fn to_file(&self, config_path: &str) -> anyhow::Result<()> {
        let f = File::create(config_path)?;
        Ok(serde_yaml::to_writer(f, self)?)
    }
}
#[derive(Debug, Default, Clone)]
pub struct Config {
    conf_pth: String,
    data_pth: String,
    /// where clash store its data
    pub clash_cfg_dir: String,
    /// where clash binary is
    pub clash_bin_pth: String,
    /// where profile stored
    pub clash_cfg_pth: String,
    /// the name of clash service
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    pub clash_srv_nam: String,
    /// whether service is running as a user instance
    #[cfg(target_os = "linux")]
    pub is_user: bool,
    timeout: Option<u64>,

    pub edit_cmd: String,
    pub open_dir_cmd: String,

    pub current_profile: core::cell::RefCell<String>,
    /// store all profile
    pub profiles: ProfileMap,
}
impl Config {
    pub fn load<P: AsRef<str>>(conf_pth: P, data_pth: P) -> anyhow::Result<Self> {
        let ConfigFile {
            basic,
            service,
            extra,
        } = ConfigFile::from_file(conf_pth.as_ref())?;
        let DataFile {
            current_profile,
            profiles,
        } = DataFile::from_file(data_pth.as_ref())?;
        let Basic {
            clash_cfg_dir,
            clash_bin_pth,
            clash_cfg_pth,
            timeout,
        } = basic;
        let Service {
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            clash_srv_nam,
            #[cfg(target_os = "linux")]
            is_user,
        } = service;
        let Extra {
            edit_cmd,
            open_dir_cmd,
        } = extra;
        Ok(Self {
            conf_pth: conf_pth.as_ref().to_string(),
            data_pth: data_pth.as_ref().to_string(),
            clash_cfg_dir,
            clash_bin_pth,
            clash_cfg_pth,
            timeout,
            edit_cmd,
            open_dir_cmd,
            current_profile,
            profiles,
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            clash_srv_nam,
            #[cfg(target_os = "linux")]
            is_user,
        })
    }

    pub fn save(self) -> anyhow::Result<()> {
        let Config {
            conf_pth,
            data_pth,
            clash_cfg_dir,
            clash_bin_pth,
            clash_cfg_pth,
            timeout,
            edit_cmd,
            open_dir_cmd,
            current_profile,
            profiles,
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            clash_srv_nam,
            #[cfg(target_os = "linux")]
            is_user,
        } = self;
        let basic = Basic {
            clash_cfg_dir,
            clash_bin_pth,
            clash_cfg_pth,
            timeout,
        };
        let service = Service {
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            clash_srv_nam,
            #[cfg(target_os = "linux")]
            is_user,
        };
        let extra = Extra {
            edit_cmd,
            open_dir_cmd,
        };
        let conf = ConfigFile {
            basic,
            service,
            extra,
        };
        conf.to_file(&conf_pth)?;
        let data = DataFile {
            current_profile,
            profiles,
        };
        data.to_file(&data_pth)?;
        Ok(())
    }

    pub fn is_valid(&self) -> bool {
        !self.clash_cfg_dir.is_empty()
            && !self.clash_cfg_pth.is_empty()
            && !self.clash_bin_pth.is_empty()
    }

    pub fn update_profile<S: AsRef<str>>(&self, profile: S) {
        *self.current_profile.borrow_mut() = profile.as_ref().to_string();
    }

    fn modify<P: AsRef<str>>(mut self, conf_pth: P, data_pth: P) -> Self {
        self.data_pth = data_pth.as_ref().to_string();
        self.conf_pth = conf_pth.as_ref().to_string();
        self
    }
}
pub fn init_config(config_dir: &std::path::PathBuf) -> anyhow::Result<()> {
    use crate::consts::{BASIC_FILE, CONFIG_FILE, DATA_FILE, DEFAULT_BASIC_CLASH_CFG_CONTENT};
    use std::fs;
    fs::create_dir_all(config_dir)?;

    ConfigFile::default().to_file(config_dir.join(CONFIG_FILE).to_str().expect(ERR_PATH_UTF_8))?;
    DataFile::default().to_file(config_dir.join(DATA_FILE).to_str().expect(ERR_PATH_UTF_8))?;

    fs::create_dir(config_dir.join("profiles"))?;
    fs::create_dir(config_dir.join("templates"))?;

    fs::write(config_dir.join(BASIC_FILE), DEFAULT_BASIC_CLASH_CFG_CONTENT)?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
/// Get necessary info
struct BasicInfo {
    #[serde(rename = "external-controller")]
    external_controller: String,
    #[serde(rename = "mixed-port")]
    mixed_port: Option<u32>,
    port: Option<u32>,
    #[serde(rename = "socks-port")]
    socks_port: Option<u32>,
    secret: Option<String>,
    #[serde(rename = "global-ua")]
    global_ua: Option<String>,
}
pub fn load_app_config(
    clashtui_dir: &std::path::Path,
    skip_init_conf: bool,
) -> anyhow::Result<(ClashUtil, Config, Option<anyhow::Error>)> {
    use crate::consts::{BASIC_FILE, CONFIG_FILE, DATA_FILE, HOST};
    let basic_clash_config_path = clashtui_dir.join(BASIC_FILE);
    let BasicInfo {
        mut external_controller,
        mixed_port,
        port,
        socks_port,
        secret,
        global_ua,
    } = serde_yaml::from_reader(
        File::open(basic_clash_config_path)
            .map_err(|e| anyhow::anyhow!("Fail to open {BASIC_FILE}:{e:?}"))?,
    )
    .map_err(|e| anyhow::anyhow!("Fail to parse {BASIC_FILE}:{e:?}"))?;
    let proxy_addr = match mixed_port.or(port).map(|p| format!("http://{HOST}:{p}")) {
        Some(s) => s,
        None => socks_port
            .map(|p| format!("socks5://{HOST}:{p}"))
            .ok_or(anyhow::anyhow!(
                "failed to load proxy_addr from {BASIC_FILE}"
            ))?,
    };
    if external_controller.starts_with("0.0.0.0") {
        external_controller = format!(
            "127.0.0.1{}",
            external_controller.strip_prefix("0.0.0.0").unwrap()
        );
    }
    let mut just_warn = None;

    let conf_pth = clashtui_dir
        .join(CONFIG_FILE)
        .to_str()
        .expect(ERR_PATH_UTF_8)
        .to_owned();
    let data_pth = clashtui_dir
        .join(DATA_FILE)
        .to_str()
        .expect(ERR_PATH_UTF_8)
        .to_owned();
    let configs = if skip_init_conf {
        match Config::load(&conf_pth, &data_pth) {
            Ok(v) => {
                if !v.is_valid() {
                    just_warn = Some(anyhow::anyhow!("Some Key Configs are missing, or Default"));
                    log::warn!("Empty Config?");
                };
                v
            }
            Err(e) => {
                just_warn = Some(anyhow::anyhow!("Fail to load configs, using Default"));
                log::error!("Unable to load config file. {e:?}");
                Config::default()
            }
        }
    } else {
        Config::default()
    }
    .modify(conf_pth, data_pth);
    log::debug!("CFG:{:?}", configs);
    let api = ClashUtil::new(
        format!("http://{external_controller}"),
        secret,
        proxy_addr,
        global_ua,
        configs.timeout,
    );
    log::debug!("API:{:?}", api);
    Ok((api, configs, just_warn))
}

#[cfg(test)]
mod test {
    use super::*;
    /*#[test]
    fn test_config() {
        let exe_dir = std::env::current_dir().unwrap();
        println!("{exe_dir:?}");
        let path_ = exe_dir
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("Example/config.yaml");
        println!("{path_:?}");
        assert!(path_.is_file());
        let path = path_.as_path().to_str().unwrap();
        let conf = Config::load(path).unwrap();
        println!("{:?}", conf);
        conf.to_file(path).unwrap();
    }*/
    #[test]
    fn test_basic_info() {
        let exe_dir = std::env::current_dir().unwrap();
        println!("{exe_dir:?}");
        let path_ = exe_dir
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("Example/basic_clash_config.yaml");
        println!("{path_:?}");
        assert!(path_.is_file());
        let path = path_.as_path().to_str().unwrap();
        let file = File::open(path).unwrap();
        let t: BasicInfo = serde_yaml::from_reader(file).unwrap();
        println!("{t:?}");
        let s = serde_yaml::to_string(&t).unwrap();
        println!("{s:?}")
    }
}
