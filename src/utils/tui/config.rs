use serde::{Deserialize, Serialize};
use std::{fs::File, io::Error};
#[derive(Debug, Deserialize, Serialize, Default)]
pub(super) struct ClashTuiConfig {
    pub clash_cfg_dir: String,
    pub clash_core_path: String,
    pub clash_cfg_path: String,
    pub clash_srv_name: String,

    pub edit_cmd: String,
    pub open_dir_cmd: String,
    pub current_profile: String,
}
impl ClashTuiConfig {
    pub fn from_file(config_path: &str) -> Result<Self, String> {
        File::open(config_path)
            .map_err(|e| e.to_string())
            .and_then(|f| serde_yaml::from_reader(f).map_err(|e| e.to_string()))
    }

    pub fn to_file(&self, config_path: &str) -> Result<(), String> {
        File::create(config_path)
            .map_err(|e| e.to_string())
            .and_then(|f| serde_yaml::to_writer(f, self).map_err(|e| e.to_string()))
    }

    pub fn check(&self) -> bool {
        self.clash_cfg_dir != "" && self.clash_cfg_path != "" && self.clash_core_path != ""
    }

    pub fn update_profile(&mut self, profile: String) {
        self.current_profile = profile;
    }
}

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

#[derive(PartialEq, Clone)]
pub enum ClashTuiConfigLoadError {
    LoadAppConfig(String),
    LoadProfileConfig(String),
    LoadClashConfig(String),
    CronUpdateProfile(String),
}
impl std::fmt::Display for ClashTuiConfigLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x = match self {
            ClashTuiConfigLoadError::LoadAppConfig(v) => v,
            ClashTuiConfigLoadError::LoadProfileConfig(v) => v,
            ClashTuiConfigLoadError::LoadClashConfig(v) => v,
            ClashTuiConfigLoadError::CronUpdateProfile(v) => v,
        };
        write!(f, "{}", x)
    }
}

pub fn init_config(
    clashtui_config_dir: &std::path::PathBuf,
    default_basic_clash_cfg_content: &str,
) -> Result<(), Error> {
    // just assume it's working, handle bug when bug occurs
    use std::fs;
    if let Err(r) = fs::create_dir_all(&clashtui_config_dir) {
        return Err(r);
    };

    if let Err(r) =
        ClashTuiConfig::default().to_file(clashtui_config_dir.join("config.yaml").to_str().unwrap())
    {
        return Err(Error::new(std::io::ErrorKind::Other, r));
    };

    if let Err(r) = fs::create_dir(clashtui_config_dir.join("profiles")) {
        return Err(r);
    };

    // Well, just keep them before I remove the template function or what
    if let Err(r) = fs::create_dir_all(clashtui_config_dir.join("templates")) {
        return Err(r);
    };

    if let Err(r) = fs::File::create(clashtui_config_dir.join("templates/template_proxy_providers"))
    {
        return Err(r);
    };

    fs::write(
        clashtui_config_dir.join("basic_clash_config.yaml"),
        default_basic_clash_cfg_content,
    )
}
