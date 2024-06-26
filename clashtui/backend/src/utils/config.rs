use api::ClashUtil;
use serde::{Deserialize, Serialize};
use std::fs::File;

#[derive(Debug, Default, Serialize, Deserialize)]
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
    File,
    Url(String),
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
pub struct Config {
    /// where clash store its data
    pub clash_cfg_dir: String,
    /// where clash binary is
    pub clash_bin_pth: String,
    /// where profile stored
    pub clash_cfg_pth: String,
    /// the name of clash service
    pub clash_srv_nam: String,
    #[cfg(target_os = "linux")]
    /// whether service is running as a user instance
    pub is_user: bool,
    timeout: Option<u64>,

    pub edit_cmd: String,
    pub open_dir_cmd: String,

    pub current_profile: core::cell::RefCell<String>,
    /// store all profile
    pub profiles: ProfileMap,
}
impl Config {
    pub fn from_file(config_path: &str) -> anyhow::Result<Self> {
        let f = File::open(config_path)?;
        Ok(serde_yaml::from_reader(f)?)
    }

    pub fn to_file(&self, config_path: &str) -> anyhow::Result<()> {
        let f = File::create(config_path)?;
        Ok(serde_yaml::to_writer(f, self)?)
    }

    pub fn is_valid(&self) -> bool {
        !self.clash_cfg_dir.is_empty()
            && !self.clash_cfg_pth.is_empty()
            && !self.clash_bin_pth.is_empty()
    }

    pub fn update_profile<S: AsRef<str>>(&self, profile: S) {
        *self.current_profile.borrow_mut() = profile.as_ref().to_string();
    }
}
pub fn init_config(config_dir: &std::path::PathBuf) -> anyhow::Result<()> {
    use crate::consts::DEFAULT_BASIC_CLASH_CFG_CONTENT;
    use std::fs;
    fs::create_dir_all(config_dir)?;

    Config::default().to_file(config_dir.join("config.yaml").to_str().unwrap())?;

    fs::create_dir(config_dir.join("profiles"))?;
    fs::create_dir(config_dir.join("templates"))?;

    fs::write(
        config_dir.join("basic_clash_config.yaml"),
        DEFAULT_BASIC_CLASH_CFG_CONTENT,
    )?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
/// Get necessary info
struct Temp {
    #[serde(rename = "external-controller")]
    external_controller: String,

    #[serde(rename = "mixed-port")]
    #[serde(skip_serializing_if = "Option::is_none")]
    mixed_port: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    port: Option<u32>,

    #[serde(rename = "socks-port")]
    #[serde(skip_serializing_if = "Option::is_none")]
    socks_port: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    secret: Option<String>,

    #[serde(rename = "global-ua")]
    #[serde(skip_serializing_if = "Option::is_none")]
    global_ua: Option<String>,
}
pub fn load_app_config(
    clashtui_dir: &std::path::Path,
    skip_init_conf: bool,
) -> anyhow::Result<(ClashUtil, Config, Option<anyhow::Error>)> {
    use crate::consts::{BASIC_FILE, HOST};
    let basic_clash_config_path = clashtui_dir.join(BASIC_FILE);
    let Temp {
        external_controller,
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
    let mut just_warn = None;

    let configs = if skip_init_conf {
        let config_path = clashtui_dir.join("config.yaml");
        match Config::from_file(config_path.to_str().unwrap()) {
            Ok(v) => {
                if !v.is_valid() {
                    just_warn = Some(anyhow::anyhow!("Some Key Configs are missing, or Default"));
                    log::warn!("Empty Config?");
                    log::debug!("{:?}", v)
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
    };
    Ok((
        ClashUtil::new(
            external_controller,
            secret,
            proxy_addr,
            global_ua,
            configs.timeout,
        ),
        configs,
        just_warn,
    ))
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
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
        let conf = Config::from_file(path).unwrap();
        println!("{:?}", conf);
        conf.to_file(path).unwrap();
    }
    #[test]
    fn test_temp() {
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
        let t: Temp = serde_yaml::from_reader(file).unwrap();
        println!("{t:?}");
        let s = serde_yaml::to_string(&t).unwrap();
        println!("{s:?}")
    }
}
