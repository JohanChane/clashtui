use serde::{Deserialize, Serialize};
/// config loaded from clash core
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ClashConfig {
    // Core infos
    pub mode: Mode,
    pub tun: TunConfig,
    // Extend infos
    pub log_level: Option<LogLevel>,
    pub bind_address: Option<String>,
    pub allow_lan: Option<bool>,
    pub ipv6: Option<bool>,
    pub global_client_fingerprint: Option<String>,
    pub tcp_concurrent: Option<bool>,
    pub global_ua: Option<String>,
    pub dns: Option<String>,
    pub geodata_mode: Option<bool>,
    pub unified_delay: Option<bool>,
    pub geo_auto_update: Option<bool>,
    pub geo_update_interval: Option<u16>,
    pub find_process_mode: Option<String>,
}
impl ClashConfig {
    pub fn build(self) -> Vec<String> {
        macro_rules! build {
            ($($value:ident),+ $(,)? $(#Option $(,)? $($option_value:ident),+)? $(,)?) => {
                vec![$(
                    format!("{}:{}", stringify!($value), $value),
                )+
                $($(
                    match $option_value {
                        Some(v) => format!("{}:{}", stringify!($option_value), v),
                        None => format!("{}:Unknown", stringify!($option_value)),
                    },
                )+)?
                ]
            };
        }
        let ClashConfig {
            mode,
            tun,
            log_level,
            bind_address,
            allow_lan,
            ipv6,
            global_client_fingerprint,
            tcp_concurrent,
            global_ua,
            dns,
            geodata_mode,
            unified_delay,
            geo_auto_update,
            geo_update_interval,
            find_process_mode,
        } = self;
        build!(mode, tun,
            #Option
            log_level,
            bind_address,
            allow_lan,
            ipv6,
            global_client_fingerprint,
            tcp_concurrent,
            global_ua,
            dns,
            geodata_mode,
            unified_delay,
            geo_auto_update,
            geo_update_interval,
            find_process_mode)
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Silent,
    Error,
    Warning,
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

#[derive(Debug, Deserialize)]
pub struct TunConfig {
    pub enable: bool,
    pub stack: TunStack,
}
impl std::fmt::Display for TunConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.enable {
            write!(f, "{}", self.stack)
        } else {
            write!(f, "Off")
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum TunStack {
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
