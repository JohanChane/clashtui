use serde_derive::{Serialize, Deserialize};
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
#[serde(default)]
pub struct ClashConfig{
    pub mixed_port:                 usize,
    pub mode:                       Mode,
    pub log_level:                  LogLevel,
    pub allow_lan:                  bool,
    bind_address:                   String,
    pub ipv6:                       bool,
    pub secret:                     String,
    tcp_concurrent:                 bool,
    pub external_controller:        String,
    pub global_client_fingerprint:  String,
    pub tun:                        TunConfig,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default] Rule,
    Global,
    Direct,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Silent,
    Error,
    Warning,
    #[default] Info,
    Debug,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TunStack {
    #[default]
    #[serde(alias = "Mixed")]
    Mixed,
    #[serde(alias = "gVisor")]
    Gvisor,
    #[serde(alias = "System")]
    System,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct TunConfig{
    pub enable: bool,
    pub stack: TunStack,
    dns_hijack: Vec<String>,
    auto_route: bool,
    auto_detect_interface: bool,
}

impl std::fmt::Display for TunStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let val = match self {
            TunStack::Mixed  => "Mixed",
            TunStack::Gvisor => "gVisor",
            TunStack::System => "System",
        };
        write!(f, "{}", val)
    }
}

impl ClashConfig {
    pub fn from_str(s:&str) -> Self{
        if s.is_empty() {
            return ClashConfig::default();
        }
        serde_json::from_str(s).unwrap()
    }
}


#[derive(Debug, Deserialize)]
pub struct ClashTuiConfig {
    #[serde(rename = "edit_cmd")]
    edit_command: String,
}

#[derive(PartialEq, Clone)]
pub enum ClashTuiConfigLoadError {
    LoadAppConfig,
    LoadProfileConfig,
}


#[test]
#[allow(unused)]
fn config(){
    use super::Clash::ClashUtil;
    let mut is = true;
    let sym = ClashUtil::new("http://127.0.0.1:9090".to_string(), "http://127.0.0.1:7890".to_string());
    match sym.config_get() {
        Ok(r) => {
            println!("{:?}", r);
            let mut t: ClashConfig = serde_json::from_str(r.as_str()).unwrap();
            let mut p = ClashConfig::default();
            
            println!("{:?}", t);
            println!("{:?}", p);
        },
        Err(e) => {
            println!("{:?}", e);
            is = false
        }       
    }
    assert!(is)
}