use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
#[serde(default)]
pub struct ClashConfig {
    pub mixed_port: usize,
    pub mode: Mode,
    pub log_level: LogLevel,
    pub allow_lan: bool,
    pub bind_address: String,
    pub ipv6: bool,
    //pub secret: String,
    pub tcp_concurrent: bool,
    //pub external_controller: String,
    pub global_client_fingerprint: String,
    pub global_ua: String,
    pub tun: TunConfig,
    pub dns: String,
    pub geodata_mode: bool,
    pub unified_delay: bool,
    pub geo_auto_update: bool,
    pub geo_update_interval: u16,
    pub find_process_mode: String,
}
impl std::str::FromStr for ClashConfig {
    type Err = std::fmt::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(std::fmt::Error);
        };
        serde_json::from_str(s).map_err(|_| std::fmt::Error)
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Rule,
    Global,
    Direct,
}
impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x = match self {
            Mode::Rule => "Rule",
            Mode::Global => "Global",
            Mode::Direct => "Direct",
        };
        write!(f, "{}", x)
    }
}
impl From<Mode> for String {
    fn from(val: Mode) -> Self {
        val.to_string()
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
 Silent,
 Error,
 Warning,
 #[default]
 Info,
 Debug,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            LogLevel::Silent => "silent",
            LogLevel::Error => "error",
            LogLevel::Warning => "warning",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TunConfig {
    pub enable: bool,
    pub stack: TunStack,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Copy)]
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
