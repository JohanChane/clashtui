use serde::{Deserialize, Serialize};
use std::fs::File;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct Basic {
    #[serde(rename = "clash_config_dir")]
    clash_cfg_dir: String,
    #[serde(rename = "clash_bin_path")]
    clash_bin_path: String,
    #[serde(rename = "clash_config_path")]
    clash_cfg_path: String,
    timeout: Option<u64>,
}
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct Extra {
    edit_cmd: String,
    open_dir_cmd: String,
}
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct Service {
    clash_srv_name: String,
    is_user: bool,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct CtCfgForUser {
    basic: Basic,
    service: Service,
    extra: Extra,
}

// ClashTui config for user
impl Default for CtCfgForUser {
    fn default() -> Self {
        CtCfgForUser {
            basic: Basic {
                clash_cfg_dir: String::from("/srv/mihomo"),
                clash_cfg_path: String::from("/srv/mihomo/config.yaml"),
                clash_bin_path: String::from("/usr/bin/mihomo"),
                timeout: None,
            },
            service: Service {
                clash_srv_name: String::from("mihomo"),
                is_user: false, // true: systemctl --user ...
            },
            extra: Extra {
                edit_cmd: String::from(""),
                open_dir_cmd: String::from(""),
            },
        }
    }
}

impl CtCfgForUser {
    pub fn load(config_path: &std::path::Path) -> Result<Self> {
        let f = File::open(config_path)?;
        Ok(serde_yml::from_reader(f)?)
    }

    pub fn save(&self, config_path: &str) -> Result<()> {
        let f = File::create(config_path)?;
        Ok(serde_yml::to_writer(f, self)?)
    }

    pub fn is_valid(&self) -> bool {
        !self.basic.clash_cfg_dir.is_empty()
            && !self.basic.clash_cfg_path.is_empty()
            && !self.basic.clash_bin_path.is_empty()
    }
}

// ClashTui config
#[derive(Debug, Default, Clone)]
pub struct CtCfg {
    /// where clash store its data
    pub clash_cfg_dir: String,
    /// where clash binary is
    pub clash_bin_path: String,
    /// where profile stored
    pub clash_cfg_path: String,
    /// the name of clash service
    pub clash_srv_name: String,
    /// whether service is running as a user instance
    pub is_user: bool,
    pub timeout: Option<u64>,

    pub edit_cmd: String,
    pub open_dir_cmd: String,
}

impl CtCfg {
    pub fn load(conf_path: &std::path::Path) -> Result<Self> {
        let CtCfgForUser {
            basic,
            service,
            extra,
        } = CtCfgForUser::load(conf_path)?;
        let Basic {
            clash_cfg_dir,
            clash_bin_path,
            clash_cfg_path,
            timeout,
        } = basic;
        let Service {
            clash_srv_name,
            is_user,
        } = service;
        let Extra {
            edit_cmd,
            open_dir_cmd,
        } = extra;
        Ok(Self {
            clash_cfg_dir,
            clash_bin_path,
            clash_cfg_path,
            timeout,
            edit_cmd,
            open_dir_cmd,
            clash_srv_name,
            is_user,
        })
    }

    pub fn save<P: AsRef<str>>(self, conf_path: P) -> Result<()> {
        let CtCfg {
            clash_cfg_dir,
            clash_bin_path,
            clash_cfg_path,
            timeout,
            edit_cmd,
            open_dir_cmd,
            clash_srv_name,
            is_user,
        } = self;
        let basic = Basic {
            clash_cfg_dir,
            clash_bin_path,
            clash_cfg_path,
            timeout,
        };
        let service = Service {
            clash_srv_name,
            is_user,
        };
        let extra = Extra {
            edit_cmd,
            open_dir_cmd,
        };
        let conf = CtCfgForUser {
            basic,
            service,
            extra,
        };
        conf.save(conf_path.as_ref())?;
        Ok(())
    }

    pub fn is_valid(&self) -> bool {
        !self.clash_cfg_dir.is_empty()
            && !self.clash_cfg_path.is_empty()
            && !self.clash_bin_path.is_empty()
    }
}

#[derive(Debug)]
pub enum ErrKind {
    IO,
    Serde,
    LoadAppConfig,
    LoadProfileConfig,
    LoadClashConfig,
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
impl From<serde_yml::Error> for CfgError {
    fn from(value: serde_yml::Error) -> Self {
        Self {
            _kind: ErrKind::Serde,
            reason: value.to_string(),
        }
    }
}
