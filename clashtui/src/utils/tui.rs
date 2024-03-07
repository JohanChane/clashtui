use core::cell::RefCell;
use core::str::FromStr as _;
use std::{
    io::Error,
    path::{Path, PathBuf},
};

use super::{
    config::{CfgError, ClashTuiConfig, ErrKind},
    parse_yaml,
    state::_State,
};
use api::{ClashConfig, ClashUtil, Resp};

pub(super) const BASIC_FILE: &str = "basic_clash_config.yaml";

pub struct ClashTuiUtil {
    pub clashtui_dir: PathBuf,
    pub(super) profile_dir: PathBuf,

    clash_api: ClashUtil,
    pub tui_cfg: ClashTuiConfig,
    clash_remote_config: RefCell<Option<ClashConfig>>,
}

// Misc
impl ClashTuiUtil {
    pub fn new(clashtui_dir: &PathBuf, is_inited: bool) -> (Self, Vec<CfgError>) {
        let ret = load_app_config(clashtui_dir, is_inited);
        let mut err_track = ret.3;
        let clash_api = ClashUtil::new(ret.1, ret.2);
        let cur_remote = match clash_api.config_get() {
            Ok(v) => v,
            Err(_) => String::new(),
        };
        let remote = match ClashConfig::from_str(cur_remote.as_str()) {
            Ok(v) => Some(v),
            Err(_) => {
                err_track.push(CfgError::new(
                    ErrKind::LoadClashConfig,
                    "Fail to load config from clash core. Is it Running?".to_string(),
                ));
                log::warn!("Fail to connect to clash. Is it Running?");
                None
            }
        };
        (
            Self {
                clashtui_dir: clashtui_dir.clone(),
                profile_dir: clashtui_dir.join("profiles").to_path_buf(),
                clash_api,
                tui_cfg: ret.0,
                clash_remote_config: RefCell::new(remote),
            },
            err_track,
        )
    }

    #[cfg(target_os = "windows")]
    pub fn update_state(
        &self,
        new_pf: Option<String>,
        new_mode: Option<String>,
        new_sysp: Option<bool>,
    ) -> _State {
        if let Some(b) = new_sysp {
            let _ = if b {
                super::ipc::enable_system_proxy(&self.clash_api.proxy_addr)
            } else {
                super::ipc::disable_system_proxy()
            };
        }
        let (pf, mode, tun) = self._update_state(new_pf, new_mode);
        let sysp = super::ipc::is_system_proxy_enabled().map_or_else(
            |v| {
                log::error!("{}", v);
                None
            },
            Some,
        );
        _State {
            profile: pf,
            mode,
            tun,
            sysproxy: sysp,
        }
    }

    #[cfg(target_os = "linux")]
    pub fn update_state(&self, new_pf: Option<String>, new_mode: Option<String>) -> _State {
        let (pf, mode, tun) = self._update_state(new_pf, new_mode);
        _State {
            profile: pf,
            mode,
            tun,
        }
    }

    pub fn fetch_recent_logs(&self, num_lines: usize) -> Vec<String> {
        let log = std::fs::read_to_string(self.clashtui_dir.join("clashtui.log"))
            .unwrap_or_else(|_| String::new());
        log.lines()
            .rev()
            .take(num_lines)
            .map(String::from)
            .collect()
    }
}
// Web
impl ClashTuiUtil {
    pub fn clash_version(&self) -> String {
        match self.clash_api.version() {
            Ok(v) => v,
            Err(e) => {
                log::warn!("{}", e);
                "Unknown".to_string()
            }
        }
    }
    fn fetch_remote(&self) -> Result<(), Error> {
        let cur_remote = self.clash_api.config_get()?;
        let remote = ClashConfig::from_str(cur_remote.as_str())
            .map_err(|_| Error::new(std::io::ErrorKind::InvalidData, "Failed to prase str"))?;
        log::debug!("{:#?}", remote);
        self.clash_remote_config.borrow_mut().replace(remote);
        log::debug!("{:#?}", self.clash_remote_config.borrow());
        Ok(())
    }
    pub fn restart_clash(&self) -> Result<String, Error> {
        self.clash_api.restart(None)
    }
    pub(super) fn dl_remote_profile(&self, url: &str) -> Result<Resp, Error> {
        self.clash_api.mock_clash_core(url)
    }
    pub(super) fn config_reload(&self, body: String) -> Result<(), Error> {
        self.clash_api.config_reload(body)
    }

    fn _update_state(
        &self,
        new_pf: Option<String>,
        new_mode: Option<String>,
    ) -> (String, Option<api::Mode>, Option<api::TunStack>) {
        if let Some(v) = new_mode {
            let load = format!(r#"{{"mode": "{}"}}"#, v);
            let _ = self
                .clash_api
                .config_patch(load)
                .map_err(|e| log::error!("Patch Errr: {}", e));
        }

        let pf = match new_pf {
            Some(v) => {
                self.tui_cfg.update_profile(&v);
                v
            }
            None => self.tui_cfg.current_profile.borrow().clone(),
        };

        if let Err(e) = self.fetch_remote() {
            if e.kind() != std::io::ErrorKind::ConnectionRefused {
                log::warn!("{}", e);
            }
        }
        let (mode, tun) = match self.clash_remote_config.borrow().as_ref() {
            Some(v) => (
                Some(v.mode),
                if v.tun.enable {
                    Some(v.tun.stack)
                } else {
                    None
                },
            ),
            None => (None, None),
        };
        (pf, mode, tun)
    }
}

fn load_app_config(
    clashtui_dir: &PathBuf,
    skip_init_conf: bool,
) -> (ClashTuiConfig, String, String, Vec<CfgError>) {
    let mut err_collect = Vec::new();
    let basic_clash_config_path = Path::new(clashtui_dir).join(BASIC_FILE);
    let basic_clash_config_value: serde_yaml::Value =
        match parse_yaml(basic_clash_config_path.as_path()) {
            Ok(r) => r,
            Err(_) => {
                err_collect.push(CfgError::new(
                    ErrKind::LoadProfileConfig,
                    "Fail to load User Defined Config".to_string(),
                ));
                serde_yaml::Value::Null
            }
        };
    let controller_api = basic_clash_config_value
        .get("external-controller")
        .and_then(|v| {
            format!(
                "http://{}",
                v.as_str().expect("external-controller not str?")
            )
            .into()
        })
        .unwrap_or_else(|| panic!("No external-controller in {BASIC_FILE}"));
    log::debug!("controller_api: {}", controller_api);

    let proxy_addr = get_proxy_addr(&basic_clash_config_value);
    log::debug!("proxy_addr: {}", proxy_addr);

    let configs = if skip_init_conf {
        let config_path = clashtui_dir.join("config.yaml");
        match ClashTuiConfig::from_file(config_path.to_str().unwrap()) {
            Ok(v) => {
                if !v.check() {
                    err_collect.push(CfgError::new(
                        ErrKind::LoadAppConfig,
                        "Some Key Configs are missing, or Default".to_string(),
                    ));
                    log::warn!("Empty Config?");
                    log::debug!("{:?}", v)
                };
                v
            }
            Err(e) => {
                err_collect.push(CfgError::new(
                    ErrKind::LoadAppConfig,
                    "Fail to load configs, using Default".to_string(),
                ));
                log::error!("Unable to load config file. {}", e);
                ClashTuiConfig::default()
            }
        }
    } else {
        ClashTuiConfig::default()
    };

    (configs, controller_api, proxy_addr, err_collect)
}

fn get_proxy_addr(yaml_data: &serde_yaml::Value) -> String {
    let host = "127.0.0.1";
    if let Some(port) = yaml_data.get("mixed-port").and_then(|v| v.as_u64()) {
        return format!("http://{}:{}", host, port);
    }
    if let Some(port) = yaml_data.get("port").and_then(|v| v.as_u64()) {
        return format!("http://{}:{}", host, port);
    }
    if let Some(port) = yaml_data.get("socks-port").and_then(|v| v.as_u64()) {
        return format!("socks5://{}:{}", host, port);
    }
    panic!("No prots in {BASIC_FILE}")
}
