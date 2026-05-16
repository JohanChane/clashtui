use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CoreType {
    #[serde(rename = "mihomo")]
    Mihomo,
    #[serde(rename = "singbox")]
    Singbox,
}
impl Default for CoreType {
    fn default() -> Self {
        Self::Mihomo
    }
}
impl std::fmt::Display for CoreType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreType::Mihomo => write!(f, "mihomo"),
            CoreType::Singbox => write!(f, "sing-box"),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct CoreConfig {
    pub config_dir: String,
    pub bin_path: String,
    pub config_path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct CoreServiceConfig {
    pub service_name: String,
    /// Controls --user flag (systemd user service) and whether sudo prefix is needed.
    /// When false, `sudo -n true` is used to detect NOPASSWD and skip password prompt.
    pub is_user: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct MihomoSection {
    #[serde(default)]
    pub core: CoreConfig,
    #[serde(default)]
    pub core_service: CoreServiceConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct SingboxSection {
    #[serde(default)]
    pub core: CoreConfig,
    #[serde(default)]
    pub core_service: CoreServiceConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ConfigFile {
    pub mihomo: MihomoSection,
    pub singbox: SingboxSection,
    pub timeout: Option<u64>,
    pub extra: Extra,
}
impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            mihomo: MihomoSection {
                core: CoreConfig {
                    config_dir: "/opt/clashtui/mihomo/config".into(),
                    bin_path: "/opt/clashtui/mihomo/mihomo".into(),
                    config_path: "/opt/clashtui/mihomo/config/config.yaml".into(),
                },
                core_service: CoreServiceConfig {
                    service_name: "clashtui_mihomo".into(),
                    is_user: false,
                },
            },
            singbox: SingboxSection {
                core: CoreConfig {
                    config_dir: "/opt/clashtui/sing-box/config".into(),
                    bin_path: "/opt/clashtui/sing-box/sing-box".into(),
                    config_path: "/opt/clashtui/sing-box/config/config.json".into(),
                },
                core_service: CoreServiceConfig {
                    service_name: "clashtui_singbox".into(),
                    is_user: false,
                },
            },
            timeout: Default::default(),
            extra: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Extra {
    pub edit_cmd: String,
    pub open_dir_cmd: String,
}
impl Default for Extra {
    fn default() -> Self {
        let common_cmd = if cfg!(windows) {
            "start %s"
        } else if cfg!(target_os = "macos") {
            "open %s"
        } else {
            "xdg-open %s"
        };
        Self {
            edit_cmd: common_cmd.to_owned(),
            open_dir_cmd: common_cmd.to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum ServiceController {
    Systemd,
    Nssm,
    OpenRc,
    Launchd,
}
impl Default for ServiceController {
    fn default() -> Self {
        if cfg!(windows) {
            ServiceController::Nssm
        } else if cfg!(target_os = "macos") {
            ServiceController::Launchd
        } else {
            ServiceController::Systemd
        }
    }
}
impl ServiceController {
    pub fn args<'a>(
        &self,
        work_type: &'a str,
        service_name: &'a str,
        is_user: bool,
    ) -> Vec<&'a str> {
        match self {
            ServiceController::Systemd if is_user => vec!["--user", work_type, service_name],
            ServiceController::Systemd => vec![work_type, service_name],
            ServiceController::OpenRc if is_user => vec![service_name, work_type, "--user"],
            ServiceController::OpenRc => vec![service_name, work_type],
            ServiceController::Nssm => vec![work_type, service_name],
            // Launchd args are constructed inline in svc_operation
            ServiceController::Launchd => vec![],
        }
    }
    pub fn bin_name(&self) -> &'static str {
        match self {
            ServiceController::Systemd => "systemctl",
            ServiceController::Nssm => "nssm",
            ServiceController::OpenRc => "rc-service",
            ServiceController::Launchd => "launchctl",
        }
    }
}

#[derive(Debug, serde:: Deserialize)]
#[serde(rename_all = "kebab-case")]
/// Get necessary info
pub struct BasicInfo {
    external_controller: String,
    mixed_port: Option<u32>,
    port: Option<u32>,
    socks_port: Option<u32>,
    pub secret: Option<String>,
    pub global_ua: Option<String>,
}

impl BasicInfo {
    const LOCALHOST: &str = "127.0.0.1";
    pub const DEFAULT: &str = "external-controller: 127.0.0.1:9090\nmixed-port: 7890";

    pub fn get_external_controller(&self) -> String {
        let str = match self.external_controller.strip_prefix("http://") {
            Some(str) => str,
            None => self.external_controller.as_str(),
        };
        if let Some(after) = str.strip_prefix("0.0.0.0") {
            format!("http://{}{}", Self::LOCALHOST, after)
        } else {
            format!("http://{}", str)
        }
    }
    pub fn get_proxy_addr(&self) -> Option<String> {
        match self.mixed_port.or(self.port) {
            Some(p) => Some(format!("http://{}:{p}", Self::LOCALHOST)),
            None => self
                .socks_port
                .map(|p| format!("socks5://{}:{p}", Self::LOCALHOST)),
        }
    }
}

#[cfg(feature = "migration_v0_3_0")]
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Basic {
    pub clash_config_dir: String,
    pub clash_bin_path: String,
    pub clash_config_path: String,
}

#[cfg(feature = "migration_v0_3_0")]
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Service {
    #[serde(alias = "clash_srv_name")]
    pub clash_service_name: String,
    pub singbox_service_name: String,
    pub is_user: bool,
    pub singbox_is_user: bool,
}

#[cfg(feature = "migration_v0_3_0")]
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Hack {
    pub service_controller: ServiceController,
}

#[cfg(feature = "migration_v0_3_0")]
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SingboxBasic {
    pub singbox_bin_path: String,
    pub singbox_config_dir: String,
    pub singbox_config_path: String,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn config_file_roundtrip() {
        let yaml = r#"mihomo:
  core:
    config_dir: /opt/clashtui/mihomo/config
    bin_path: /opt/clashtui/mihomo/mihomo
    config_path: /opt/clashtui/mihomo/config/config.yaml
  core_service:
    service_name: clashtui_mihomo
    is_user: false
singbox:
  core:
    bin_path: /opt/clashtui/sing-box/sing-box
    config_dir: /opt/clashtui/sing-box/config
    config_path: /opt/clashtui/sing-box/config/config.json
  core_service:
    service_name: clashtui_singbox
    is_user: false
timeout: 5
extra:
  edit_cmd: kitty -e nvim "%s"
  open_dir_cmd: kitty -e yazi "%s"
"#;
        let cfg: ConfigFile = serde_yml::from_str(yaml).unwrap();
        assert_eq!(cfg.mihomo.core.config_dir, "/opt/clashtui/mihomo/config");
        assert_eq!(cfg.mihomo.core.bin_path, "/opt/clashtui/mihomo/mihomo");
        assert_eq!(cfg.mihomo.core.config_path, "/opt/clashtui/mihomo/config/config.yaml");
        assert_eq!(cfg.mihomo.core_service.service_name, "clashtui_mihomo");
        assert!(!cfg.mihomo.core_service.is_user);
        assert_eq!(cfg.singbox.core.bin_path, "/opt/clashtui/sing-box/sing-box");
        assert_eq!(cfg.singbox.core.config_dir, "/opt/clashtui/sing-box/config");
        assert_eq!(cfg.singbox.core.config_path, "/opt/clashtui/sing-box/config/config.json");
        assert_eq!(cfg.singbox.core_service.service_name, "clashtui_singbox");
        assert_eq!(cfg.timeout, Some(5));
        assert_eq!(cfg.extra.edit_cmd, r#"kitty -e nvim "%s""#);

        let serialized = serde_yml::to_string(&cfg).unwrap();
        let deser: ConfigFile = serde_yml::from_str(&serialized).unwrap();
        assert_eq!(deser.mihomo.core.config_dir, cfg.mihomo.core.config_dir);
        assert_eq!(deser.singbox.core.bin_path, cfg.singbox.core.bin_path);
    }

    #[test]
    fn config_file_defaults() {
        let yaml = "mihomo: {}\nsingbox: {}";
        let cfg: ConfigFile = serde_yml::from_str(yaml).unwrap();
        assert_eq!(cfg.mihomo.core.config_dir, "");
        assert_eq!(cfg.mihomo.core_service.service_name, "");
        assert!(!cfg.mihomo.core_service.is_user);
        assert_eq!(cfg.timeout, None);
    }

    #[test]
    fn core_profile_data_serde() {
        let yaml = r#"cur_profile: my
profiles:
  pf1:
    dtype: File
    no_pp: true
  pf2:
    dtype: !Url "https://example.com"
    no_pp: false
"#;
        let data: super::super::database::CoreProfileData = serde_yml::from_str(yaml).unwrap();
        assert_eq!(data.cur_profile.as_deref(), Some("my"));
        assert_eq!(data.profiles.get("pf1").unwrap().no_pp, true);
        assert_eq!(data.profiles.get("pf2").unwrap().no_pp, false);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn extra_default_macos_uses_open() {
        let extra = Extra::default();
        assert_eq!(extra.edit_cmd, "open %s");
        assert_eq!(extra.open_dir_cmd, "open %s");
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn extra_default_non_macos_not_open() {
        let extra = Extra::default();
        // On non-macOS platforms, the default should NOT be "open %s"
        assert_ne!(extra.edit_cmd, "open %s");
        assert_ne!(extra.open_dir_cmd, "open %s");
    }

    #[test]
    fn service_controller_bin_name() {
        assert_eq!(ServiceController::Launchd.bin_name(), "launchctl");
        assert_eq!(ServiceController::Systemd.bin_name(), "systemctl");
        assert_eq!(ServiceController::Nssm.bin_name(), "nssm");
        assert_eq!(ServiceController::OpenRc.bin_name(), "rc-service");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn service_controller_default_macos_is_launchd() {
        assert_eq!(ServiceController::default(), ServiceController::Launchd);
    }

    #[test]
    fn service_controller_args_nssm() {
        let args = ServiceController::Nssm.args("start", "svc", false);
        assert_eq!(args, vec!["start", "svc"]);
    }

    #[test]
    fn service_controller_args_systemd_user() {
        let args = ServiceController::Systemd.args("stop", "svc", true);
        assert_eq!(args, vec!["--user", "stop", "svc"]);
    }

    #[test]
    fn service_controller_args_systemd_system() {
        let args = ServiceController::Systemd.args("stop", "svc", false);
        assert_eq!(args, vec!["stop", "svc"]);
    }

    #[test]
    fn service_controller_args_openrc_user() {
        let args = ServiceController::OpenRc.args("restart", "svc", true);
        assert_eq!(args, vec!["svc", "restart", "--user"]);
    }

    #[test]
    fn service_controller_args_openrc_system() {
        let args = ServiceController::OpenRc.args("restart", "svc", false);
        assert_eq!(args, vec!["svc", "restart"]);
    }

    #[test]
    fn service_controller_args_launchd_is_empty() {
        let args = ServiceController::Launchd.args("start", "svc", false);
        assert!(args.is_empty());
        let args = ServiceController::Launchd.args("restart", "svc", true);
        assert!(args.is_empty());
    }
}
