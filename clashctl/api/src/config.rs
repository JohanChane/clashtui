use serde::{Deserialize, Serialize};
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
