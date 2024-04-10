use core::cell::OnceCell;
use core::str::FromStr as _;
use std::{
    io::Error,
    path::{Path, PathBuf},
};
mod impl_app;
mod impl_clashsrv;
mod impl_profile;

use super::{
    config::{CfgError, ClashTuiConfig, ErrKind},
    parse_yaml,
};
use api::{ClashConfig, ClashUtil, Resp};

const BASIC_FILE: &str = "basic_clash_config.yaml";

pub struct ClashBackend {
    pub clashtui_dir: PathBuf,
    profile_dir: PathBuf,

    clash_api: ClashUtil,
    pub tui_cfg: ClashTuiConfig,
    clash_remote_config: OnceCell<ClashConfig>,
}

// Misc
impl ClashBackend {
    pub fn new(clashtui_dir: &PathBuf, is_inited: bool) -> (Self, Vec<CfgError>) {
        let ret = load_app_config(clashtui_dir, is_inited);
        let mut err_track = ret.2;
        let clash_api = ret.1;
        let clash_remote_config = OnceCell::default();
        match ClashConfig::from_str(clash_api.config_get().unwrap_or_default().as_str()) {
            // That must be empty, call unwrap is fine
            Ok(remote) => clash_remote_config.set(remote).unwrap(),
            Err(_) => {
                err_track.push(CfgError::new(
                    ErrKind::LoadClashConfig,
                    "Fail to load config from clash core. Is it Running?".to_string(),
                ));
                log::warn!("Fail to connect to clash. Is it Running?")
            }
        }
        (
            Self {
                clashtui_dir: clashtui_dir.clone(),
                profile_dir: clashtui_dir.join("profiles").to_path_buf(),
                clash_api,
                tui_cfg: ret.0,
                clash_remote_config,
            },
            err_track,
        )
    }
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
        let _ = self.clash_remote_config.set(remote);
        Ok(())
    }
    pub fn restart_clash(&self) -> Result<String, Error> {
        self.clash_api.restart(None)
    }
    fn dl_remote_profile(&self, url: &str) -> Result<Resp, Error> {
        self.clash_api.mock_clash_core(url)
    }
    fn config_reload(&self, body: String) -> Result<(), Error> {
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
        let (mode, tun) = match self.clash_remote_config.get() {
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
) -> (ClashTuiConfig, ClashUtil, Vec<CfgError>) {
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

    let secret = basic_clash_config_value
        .get("secret")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

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
    (
        configs,
        ClashUtil::new(controller_api, secret, proxy_addr),
        err_collect,
    )
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
