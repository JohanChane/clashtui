use serde::{Deserialize, Serialize};
use std::fs::File;
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub clash_cfg_dir: String,
    pub clash_core_path: String,
    pub clash_cfg_path: String,
    pub clash_srv_name: String,
    pub is_user: bool,

    pub edit_cmd: String,
    pub open_dir_cmd: String,

    pub current_profile: core::cell::RefCell<String>,

    pub profiles: core::cell::RefCell<std::collections::HashMap<String, String>>,
}
impl Config {
    pub fn from_file(config_path: &str) -> Result<Self> {
        let f = File::open(config_path)?;
        Ok(serde_yaml::from_reader(f)?)
    }

    pub fn to_file(&self, config_path: &str) -> Result<()> {
        let f = File::create(config_path)?;
        Ok(serde_yaml::to_writer(f, self)?)
    }

    pub fn check(&self) -> bool {
        !self.clash_cfg_dir.is_empty()
            && !self.clash_cfg_path.is_empty()
            && !self.clash_core_path.is_empty()
    }

    pub fn update_profile(&self, profile: &String) {
        self.current_profile.borrow_mut().clone_from(profile);
    }
}
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_save_and_load() {
        let exe_dir = std::env::current_dir().unwrap();
        println!("{exe_dir:?}");
        let path_ = exe_dir.parent().unwrap().join("Example/config.yaml");
        println!("{path_:?}");
        assert!(path_.is_file());
        let path = path_.as_path().to_str().unwrap();
        let conf = Config::from_file(path).unwrap();
        println!("{:?}", conf);
        conf.to_file(path).unwrap();
    }
}
#[derive(Debug)]
pub enum ErrKind {
    IO,
    Serde,
    LoadAppConfig,
    LoadProfileConfig,
    LoadClashConfig,
}
type Result<T> = core::result::Result<T, CfgError>;
#[derive(Debug)]
pub struct CfgError {
    _kind: ErrKind,
    pub reason: String,
}
impl CfgError {
    pub fn new(_kind: ErrKind, reason: String) -> Self {
        Self { _kind, reason }
    }
}
impl core::fmt::Display for CfgError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:#?}", self)
    }
}
impl std::error::Error for CfgError {}
impl From<std::io::Error> for CfgError {
    fn from(value: std::io::Error) -> Self {
        Self {
            _kind: ErrKind::IO,
            reason: value.to_string(),
        }
    }
}
impl From<serde_yaml::Error> for CfgError {
    fn from(value: serde_yaml::Error) -> Self {
        Self {
            _kind: ErrKind::Serde,
            reason: value.to_string(),
        }
    }
}
pub fn init_config(config_dir: &std::path::PathBuf) -> Result<()> {
    const DEFAULT_BASIC_CLASH_CFG_CONTENT: &str = r#"mixed-port: 7890
mode: rule
log-level: info
external-controller: 127.0.0.1:9090"#;
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
