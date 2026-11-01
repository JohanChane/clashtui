use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum CoreType {
    #[serde(rename = "mihomo")]
    #[default]
    Mihomo,
    #[serde(rename = "singbox")]
    Singbox,
}

impl CoreType {
    pub const ALL: [Self; 2] = [Self::Mihomo, Self::Singbox];

    /// Stable configuration and script identifier, independent of the display name.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Mihomo => "mihomo",
            Self::Singbox => "singbox",
        }
    }

    pub const fn index(self) -> usize {
        match self {
            Self::Mihomo => 0,
            Self::Singbox => 1,
        }
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
}
impl Default for ConfigFile {
    fn default() -> Self {
        #[cfg(windows)]
        {
            let local_appdata = std::env::var("LOCALAPPDATA")
                .unwrap_or_else(|_| {
                    let home = std::env::var("USERPROFILE").unwrap_or_default();
                    format!("{home}/AppData/Local")
                })
                .replace('\\', "/");
            let base = format!("{local_appdata}/clashtui");
            Self {
                mihomo: MihomoSection {
                    core: CoreConfig {
                        config_dir: format!("{base}/mihomo/config"),
                        bin_path: format!("{base}/mihomo/mihomo.exe"),
                        config_path: format!("{base}/mihomo/config/config.yaml"),
                    },
                    core_service: CoreServiceConfig {
                        service_name: "clashtui_mihomo".into(),
                    },
                },
                singbox: SingboxSection {
                    core: CoreConfig {
                        config_dir: format!("{base}/sing-box/config"),
                        bin_path: format!("{base}/sing-box/sing-box.exe"),
                        config_path: format!("{base}/sing-box/config/config.json"),
                    },
                    core_service: CoreServiceConfig {
                        service_name: "clashtui_singbox".into(),
                    },
                },
                timeout: Default::default(),
            }
        }
        #[cfg(not(windows))]
        Self {
            mihomo: MihomoSection {
                core: CoreConfig {
                    config_dir: "/opt/clashtui/mihomo/config".into(),
                    bin_path: "/opt/clashtui/mihomo/mihomo".into(),
                    config_path: "/opt/clashtui/mihomo/config/config.yaml".into(),
                },
                core_service: CoreServiceConfig {
                    service_name: "clashtui_mihomo".into(),
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
                },
            },
            timeout: Default::default(),
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
singbox:
  core:
    bin_path: /opt/clashtui/sing-box/sing-box
    config_dir: /opt/clashtui/sing-box/config
    config_path: /opt/clashtui/sing-box/config/config.json
  core_service:
    service_name: clashtui_singbox
timeout: 5
"#;
        let cfg: ConfigFile = serde_yml::from_str(yaml).unwrap();
        assert_eq!(cfg.mihomo.core.config_dir, "/opt/clashtui/mihomo/config");
        assert_eq!(cfg.mihomo.core.bin_path, "/opt/clashtui/mihomo/mihomo");
        assert_eq!(
            cfg.mihomo.core.config_path,
            "/opt/clashtui/mihomo/config/config.yaml"
        );
        assert_eq!(cfg.mihomo.core_service.service_name, "clashtui_mihomo");
        assert_eq!(cfg.singbox.core.bin_path, "/opt/clashtui/sing-box/sing-box");
        assert_eq!(cfg.singbox.core.config_dir, "/opt/clashtui/sing-box/config");
        assert_eq!(
            cfg.singbox.core.config_path,
            "/opt/clashtui/sing-box/config/config.json"
        );
        assert_eq!(cfg.singbox.core_service.service_name, "clashtui_singbox");
        assert_eq!(cfg.timeout, Some(5));

        let serialized = serde_yml::to_string(&cfg).unwrap();
        let deser: ConfigFile = serde_yml::from_str(&serialized).unwrap();
        assert_eq!(deser.mihomo.core.config_dir, cfg.mihomo.core.config_dir);
        assert_eq!(deser.singbox.core.bin_path, cfg.singbox.core.bin_path);
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
        assert!(data.profiles.get("pf1").unwrap().no_pp);
        assert!(!data.profiles.get("pf2").unwrap().no_pp);
    }

    fn fixture_path(name: &str) -> String {
        format!(
            "{}/tests/clashtui_configs/{}",
            env!("CARGO_MANIFEST_DIR"),
            name
        )
    }

    fn load_config_fixture(name: &str) -> ConfigFile {
        let path = fixture_path(name);
        let yaml = std::fs::read_to_string(&path).unwrap();
        serde_yml::from_str(&yaml).unwrap()
    }

    #[test]
    fn config_fixture_mihomo_full() {
        let cfg = load_config_fixture("mihomo/full.yaml");
        assert_eq!(cfg.mihomo.core.config_dir, "/opt/clashtui/mihomo/config");
        assert_eq!(cfg.mihomo.core.bin_path, "/opt/clashtui/mihomo/mihomo");
        assert_eq!(
            cfg.mihomo.core.config_path,
            "/opt/clashtui/mihomo/config/config.yaml"
        );
        assert_eq!(cfg.mihomo.core_service.service_name, "clashtui_mihomo");
        assert_eq!(cfg.timeout, Some(5));
    }

    #[test]
    fn config_fixture_mihomo_full_roundtrip() {
        let cfg = load_config_fixture("mihomo/full.yaml");
        let serialized = serde_yml::to_string(&cfg).unwrap();
        let deser: ConfigFile = serde_yml::from_str(&serialized).unwrap();
        assert_eq!(deser.mihomo.core.config_dir, cfg.mihomo.core.config_dir);
        assert_eq!(deser.mihomo.core.bin_path, cfg.mihomo.core.bin_path);
        assert_eq!(deser.singbox.core.bin_path, cfg.singbox.core.bin_path);
        assert_eq!(deser.timeout, cfg.timeout);
    }

    #[test]
    fn config_fixture_empty_sections_defaults() {
        let cfg = load_config_fixture("mihomo/empty_sections.yaml");
        assert_eq!(cfg.mihomo.core.config_dir, "");
        assert_eq!(cfg.mihomo.core.bin_path, "");
        assert_eq!(cfg.mihomo.core.config_path, "");
        assert_eq!(cfg.mihomo.core_service.service_name, "");
        assert_eq!(cfg.singbox.core.config_dir, "");
        assert_eq!(cfg.singbox.core.bin_path, "");
        assert_eq!(cfg.singbox.core.config_path, "");
        assert_eq!(cfg.singbox.core_service.service_name, "");
        assert_eq!(cfg.timeout, None);
    }

    #[test]
    fn config_fixture_null_optionals() {
        let cfg = load_config_fixture("mihomo/null_optionals.yaml");
        assert_eq!(cfg.mihomo.core.config_dir, "/opt/mihomo");
        assert_eq!(cfg.mihomo.core_service.service_name, "mihomo_svc");
        assert_eq!(cfg.timeout, None);
    }

    #[test]
    fn config_fixture_singbox_full() {
        let cfg = load_config_fixture("sing-box/full.yaml");
        assert_eq!(cfg.singbox.core.config_dir, "/opt/clashtui/sing-box/config");
        assert_eq!(cfg.singbox.core.bin_path, "/opt/clashtui/sing-box/sing-box");
        assert_eq!(
            cfg.singbox.core.config_path,
            "/opt/clashtui/sing-box/config/config.json"
        );
        assert_eq!(cfg.singbox.core_service.service_name, "clashtui_singbox");
        assert_eq!(cfg.timeout, Some(30));
    }

    #[test]
    fn config_fixture_singbox_minimal_empty_strings() {
        let cfg = load_config_fixture("sing-box/minimal.yaml");
        assert_eq!(cfg.mihomo.core.config_dir, "");
        assert_eq!(cfg.mihomo.core.bin_path, "");
        assert_eq!(cfg.mihomo.core.config_path, "");
        assert_eq!(cfg.mihomo.core_service.service_name, "");
        assert_eq!(cfg.singbox.core.config_dir, "");
        assert_eq!(cfg.singbox.core.bin_path, "");
        assert_eq!(cfg.singbox.core.config_path, "");
        assert_eq!(cfg.singbox.core_service.service_name, "");
        assert_eq!(cfg.timeout, None);
    }
}
