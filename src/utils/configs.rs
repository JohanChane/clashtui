use serde_derive::{Deserialize, Serialize};
use std::{fmt::Display, fs::File, io::Error};
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
#[serde(default)]
pub struct ClashConfig {
    // pub mixed_port: usize,
    pub mode: Mode,
    // pub log_level: LogLevel,
    // pub allow_lan: bool,
    // bind_address: String,
    // pub ipv6: bool,
    // pub secret: String,
    // tcp_concurrent: bool,
    // pub external_controller: String,
    // pub global_client_fingerprint: String,
    pub tun: TunConfig,
}
impl ClashConfig {
    pub fn from_str(s: &str) -> Option<Self> {
        if s.is_empty() {
            return None;
        }
        serde_json::from_str(s).unwrap()
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Rule,
    Global,
    Direct,
}
impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x = match self {
            Mode::Rule => "Rule",
            Mode::Global => "Global",
            Mode::Direct => "Direct",
        };
        write!(f, "{}", x)
    }
}
// #[derive(Debug, Serialize, Deserialize, Default)]
// #[serde(rename_all = "lowercase")]
// pub enum LogLevel {
//     Silent,
//     Error,
//     Warning,
//     #[default]
//     Info,
//     Debug,
// }

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TunConfig {
    pub enable: bool,
    pub stack: TunStack,
}
#[derive(Eq, Hash, PartialEq)]
pub enum Flags {
    FirstInit,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub enum TunStack {
    #[default]
    #[serde(alias = "Mixed")]
    Mixed,
    #[serde(alias = "gVisor")]
    Gvisor,
    #[serde(alias = "System")]
    System,
}
impl std::fmt::Display for TunStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let val = match self {
            TunStack::Mixed => "Mixed",
            TunStack::Gvisor => "gVisor",
            TunStack::System => "System",
        };
        write!(f, "{}", val)
    }
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ClashTuiConfig {
    pub clash_cfg_dir: String,
    pub clash_core_path: String,
    pub clash_cfg_path: String,
    pub clash_srv_name: String,

    //#[serde(rename = "edit_cmd")]
    //pub edit_command: String,
    #[serde(rename = "current_profile")]
    pub cur_profile: String,
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
        if self.clash_cfg_dir == "" {
            return false;
        }
        if self.clash_cfg_path == "" {
            return false;
        }
        if self.clash_core_path == "" {
            return false;
        }
        true
    }

    pub fn update_profile(&mut self, profile: String) {
        self.cur_profile = profile;
    }
}

#[test]
fn test_save_and_load() {
    let mut flag = true;
    let path = "/root/.config/clashtui/config.yaml";
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
    LoadAppConfig(Box<str>),
    LoadProfileConfig(Box<str>),
    LoadClashConfig(Box<str>),
}

pub fn init_config(
    clashtui_config_dir: &std::path::PathBuf,
    symbols: &crate::ui::SharedSymbols,
) -> Result<(), Error> {
    // just assume it's working, handle bug when bug occurs
    use std::fs;
    let r = fs::create_dir_all(&clashtui_config_dir);
    if r.is_err() {
        return r;
    }
    let r = ClashTuiConfig::default()
        .to_file(clashtui_config_dir.join("config.yaml").to_str().unwrap());
    if r.is_err() {
        return Err(Error::new(std::io::ErrorKind::Other, r.err().unwrap()));
    }
    let r = fs::create_dir(clashtui_config_dir.join("profiles"));
    if r.is_err() {
        return r;
    }
    // Well, just keep them before I remove the template function or what
    let r = fs::create_dir_all(clashtui_config_dir.join("templates"));
    if r.is_err() {
        return r;
    }
    let r = fs::File::create(clashtui_config_dir.join("templates/template_proxy_providers"));
    match r {
        Err(e) => return Err(e),
        Ok(_) => (),
    };

    fs::write(
        clashtui_config_dir.join("basic_clash_config.yaml"),
        &symbols.default_basic_clash_cfg_content,
    )
}

#[test]
#[allow(unused)]
fn config() {
    use super::clash::ClashUtil;
    let mut is = true;
    let sym = ClashUtil::new(
        "http://127.0.0.1:9090".to_string(),
        "http://127.0.0.1:7890".to_string(),
    );
    match sym.config_get() {
        Ok(r) => {
            println!("{:?}", r);
            let mut t: ClashConfig = serde_json::from_str(r.as_str()).unwrap();
            let mut p = ClashConfig::default();

            println!("{:?}", t);
            println!("{:?}", p);
        }
        Err(e) => {
            println!("{:?}", e);
            is = false
        }
    }
    assert!(is)
}
