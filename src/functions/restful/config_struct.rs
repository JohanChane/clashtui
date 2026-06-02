use serde::{Deserialize, Serialize};
/// config loaded from clash core (mihomo or sing-box)
///
/// Fields present in both cores are always displayed if available.
/// Core-specific fields are Optional and only shown when the API returns them.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ClashConfig {
    pub mode: Mode,
    #[serde(default)]
    pub tun: Option<TunConfig>,
    // Common (both cores)
    pub log_level: Option<LogLevel>,
    pub bind_address: Option<String>,
    pub allow_lan: Option<bool>,
    pub ipv6: Option<bool>,
    // Mihomo-specific
    pub global_client_fingerprint: Option<String>,
    pub tcp_concurrent: Option<bool>,
    pub global_ua: Option<String>,
    pub dns: Option<String>,
    pub geodata_mode: Option<bool>,
    pub unified_delay: Option<bool>,
    pub geo_auto_update: Option<bool>,
    pub geo_update_interval: Option<u16>,
    pub find_process_mode: Option<String>,
    // sing-box-specific
    pub port: Option<u16>,
    pub socks_port: Option<u16>,
    pub redir_port: Option<u16>,
    pub tproxy_port: Option<u16>,
    pub mixed_port: Option<u16>,
    pub mode_list: Option<Vec<String>>,
}
impl ClashConfig {
    pub fn build(&self) -> Vec<String> {
        let mut lines = vec![format!("mode:{}", self.mode)];

        if let Some(ref v) = self.tun {
            lines.push(format!("tun:{v}"));
        }
        if let Some(ref v) = self.log_level {
            lines.push(format!("log_level:{v}"));
        }
        if let Some(ref v) = self.bind_address {
            lines.push(format!("bind_address:{v}"));
        }
        if let Some(v) = self.allow_lan {
            lines.push(format!("allow_lan:{v}"));
        }
        if let Some(v) = self.ipv6 {
            lines.push(format!("ipv6:{v}"));
        }
        if let Some(ref v) = self.global_client_fingerprint {
            lines.push(format!("global_client_fingerprint:{v}"));
        }
        if let Some(v) = self.tcp_concurrent {
            lines.push(format!("tcp_concurrent:{v}"));
        }
        if let Some(ref v) = self.global_ua {
            lines.push(format!("global_ua:{v}"));
        }
        if let Some(ref v) = self.dns {
            lines.push(format!("dns:{v}"));
        }
        if let Some(v) = self.geodata_mode {
            lines.push(format!("geodata_mode:{v}"));
        }
        if let Some(v) = self.unified_delay {
            lines.push(format!("unified_delay:{v}"));
        }
        if let Some(v) = self.geo_auto_update {
            lines.push(format!("geo_auto_update:{v}"));
        }
        if let Some(v) = self.geo_update_interval {
            lines.push(format!("geo_update_interval:{v}"));
        }
        if let Some(ref v) = self.find_process_mode {
            lines.push(format!("find_process_mode:{v}"));
        }
        if let Some(v) = self.port {
            lines.push(format!("port:{v}"));
        }
        if let Some(v) = self.socks_port {
            lines.push(format!("socks_port:{v}"));
        }
        if let Some(v) = self.redir_port {
            lines.push(format!("redir_port:{v}"));
        }
        if let Some(v) = self.tproxy_port {
            lines.push(format!("tproxy_port:{v}"));
        }
        if let Some(v) = self.mixed_port {
            lines.push(format!("mixed_port:{v}"));
        }
        if let Some(ref v) = self.mode_list {
            lines.push(format!("mode_list:{}", v.join(", ")));
        }

        lines
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, strum::VariantArray)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[serde(alias = "Rule")]
    Rule,
    #[serde(alias = "Global")]
    Global,
    #[serde(alias = "Direct")]
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

#[derive(Debug, Serialize, Deserialize, Clone, Copy, strum::VariantArray)]
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

#[derive(Debug, Serialize, Deserialize, Clone, Copy, strum::VariantArray)]
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

#[cfg(test)]
mod tests {
    use super::*;

    fn load_singbox_config() -> ClashConfig {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/apidata/sing-box/configs.json"
        );
        let data = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&data).unwrap()
    }

    #[test]
    fn singbox_config_mode_is_rule() {
        let cfg = load_singbox_config();
        assert!(matches!(cfg.mode, Mode::Rule));
    }

    #[test]
    fn singbox_config_tun_is_null() {
        let cfg = load_singbox_config();
        assert!(cfg.tun.is_none());
    }

    #[test]
    fn singbox_config_log_level_is_error() {
        let cfg = load_singbox_config();
        assert!(matches!(cfg.log_level, Some(LogLevel::Error)));
    }

    #[test]
    fn singbox_config_ports_are_zero() {
        let cfg = load_singbox_config();
        assert_eq!(cfg.port, Some(0));
        assert_eq!(cfg.mixed_port, Some(0));
        assert_eq!(cfg.socks_port, Some(0));
    }

    #[test]
    fn singbox_config_mode_list() {
        let cfg = load_singbox_config();
        let list = cfg.mode_list.expect("mode_list missing");
        assert_eq!(list, vec!["Rule", "Proxy", "Direct"]);
    }

    #[test]
    fn singbox_config_mihomo_fields_are_none() {
        let cfg = load_singbox_config();
        assert!(cfg.global_client_fingerprint.is_none());
        assert!(cfg.tcp_concurrent.is_none());
        assert!(cfg.global_ua.is_none());
    }

    #[test]
    fn singbox_config_build_output() {
        let cfg = load_singbox_config();
        let lines = cfg.build();
        assert!(lines.contains(&"mode:Rule".to_string()));
        assert!(lines.contains(&"mode_list:Rule, Proxy, Direct".to_string()));
        assert!(lines.contains(&"log_level:error".to_string()));
    }
}
