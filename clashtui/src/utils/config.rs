use core::cell::RefCell;
use serde::{Deserialize, Serialize};
use std::fs::File;
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ClashTuiConfig {
    pub clash_cfg_dir: String,
    pub clash_core_path: String,
    pub clash_cfg_path: String,
    pub clash_srv_name: String,

    pub edit_cmd: String,
    pub open_dir_cmd: String,

    pub current_profile: RefCell<String>,
}
impl ClashTuiConfig {
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
#[test]
#[ignore = "Not testable in Github Action, need to be fixed"]
fn test_save_and_load() {
    let mut flag = true;
    let path = "config.yaml";
    let conf = match ClashTuiConfig::from_file(path) {
        Ok(v) => v,
        Err(e) => {
            flag = false;
            println!("{}", e);
            ClashTuiConfig::default()
        }
    };
    assert!(flag);
    flag = false;
    println!("{:?}", conf);
    let e = conf.to_file(path);
    match e {
        Ok(_) => flag = true,
        Err(v) => println!("{}", v),
    };
    assert!(flag);
}

#[derive(Debug)]
pub enum ErrKind {
    IO,
    Serde,
    LoadAppConfig,
    LoadProfileConfig,
    LoadClashConfig,
    CronUpdateProfile,
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
pub fn init_config(
    clashtui_config_dir: &std::path::PathBuf,
    default_basic_clash_cfg_content: &str,
) -> Result<()> {
    // just assume it's working, handle bug when bug occurs
    use std::fs;
    fs::create_dir_all(clashtui_config_dir)?;

    ClashTuiConfig::default().to_file(clashtui_config_dir.join("config.yaml").to_str().unwrap())?;

    fs::create_dir(clashtui_config_dir.join("profiles"))?;
    // Well, just keep them before I remove the template function or what
    fs::create_dir_all(clashtui_config_dir.join("templates"))?;
    fs::File::create(clashtui_config_dir.join("templates/template_proxy_providers"))?;

    fs::write(
        clashtui_config_dir.join("basic_clash_config.yaml"),
        default_basic_clash_cfg_content,
    )?;
    Ok(())
}
