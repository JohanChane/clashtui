use core::cell::RefCell;
use core::str::FromStr as _;
use std::{
    io::Error,
    path::{Path, PathBuf},
};
use api::ProfileSectionType;

mod impl_app;
mod impl_clashsrv;
mod impl_profile;

use super::{
    config::{CfgError, ClashTuiConfig, ErrKind},
    parse_yaml,
    ClashTuiData,
};
use api::{ClashConfig, ClashUtil, Resp};

// format: {section_key: [(name, url, path)]}
pub type NetProviderMap = std::collections::HashMap<ProfileSectionType, Vec<(String, String, String)>>;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum ProfileType {
    Url,
    Yaml,
}

const BASIC_FILE: &str = "basic_clash_config.yaml";
const DATA_FILE: &str = "data.yaml";

pub struct ClashTuiUtil {
    pub clashtui_dir: PathBuf,
    profile_dir: PathBuf,

    clash_api: ClashUtil,
    pub tui_cfg: ClashTuiConfig,
    pub clashtui_data: RefCell<ClashTuiData>,
}

// Misc
impl ClashTuiUtil {
    pub fn new(clashtui_dir: &PathBuf, is_inited: bool) -> (Self, Vec<CfgError>) {
        let ret = load_app_config(clashtui_dir, is_inited);
        let mut err_track = ret.2;
        let clash_api = ret.1;

        if clash_api.version().is_err() {
            err_track.push(CfgError::new(
                ErrKind::LoadClashConfig,
                "Fail to load config from clash core. Is it Running?".to_string(),
            ));
            log::warn!("Fail to connect to clash. Is it Running?")
        }

        let data_path = clashtui_dir.join(DATA_FILE);
        let clashtui_data = RefCell::new(ClashTuiData::from_file(data_path.to_str().unwrap()).unwrap_or_default());

        (
            Self {
                clashtui_dir: clashtui_dir.clone(),
                profile_dir: clashtui_dir.join("profiles").to_path_buf(),
                clash_api,
                tui_cfg: ret.0,
                clashtui_data,
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
    fn fetch_remote(&self) -> Result<ClashConfig, Error> {
        self.clash_api.config_get().and_then(|cur_remote| {
            ClashConfig::from_str(cur_remote.as_str())
                .map_err(|_| Error::new(std::io::ErrorKind::InvalidData, "Failed to prase str"))
        })
    }
    pub fn restart_clash(&self) -> Result<String, Error> {
        self.clash_api.restart(None)
    }
    fn dl_remote_profile(&self, url: &str) -> Result<Resp, Error> {
        let with_proxy = self.clash_api.version().is_ok() && self.clash_api.check_connectivity().is_ok();
        let timeout = self.tui_cfg.timeout;
        self.clash_api.mock_clash_core(url, with_proxy, timeout)
    }
    fn config_reload(&self, body: String) -> Result<(), Error> {
        self.clash_api.config_reload(body)
    }

    pub fn save_to_data_file(&self) {
        let data_path = self.clashtui_dir.join(DATA_FILE);
        let _ = self.clashtui_data.borrow_mut().to_file(data_path.to_str().unwrap());
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
                let controller_address = v.as_str().expect("external-controller not str?");
                if controller_address.starts_with("0.0.0.0:") {
                    // replace 0.0.0.0 with 127.0.0.1
                    let port_index = controller_address.find(':').unwrap_or(0) + 1;
                    let port = &controller_address[port_index..];
                    let machine_ip = "127.0.0.1";
                    Some(format!("http://{machine_ip}:{port}"))
                } else {
                    Some(format!("http://{}", controller_address))
                }
        })
        .unwrap_or_else(|| panic!("No external-controller in {BASIC_FILE}"));
    log::info!("controller_api: {}", controller_api);

    let proxy_addr = get_proxy_addr(&basic_clash_config_value);
    log::info!("proxy_addr: {}", proxy_addr);

    let secret = basic_clash_config_value
        .get("secret")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let clash_ua = basic_clash_config_value
        .get("global-ua")
        .and_then(|v| v.as_str())
        .unwrap_or("clash.meta")
        .to_string();
    log::info!("clash_ua: {}", clash_ua);

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
        ClashUtil::new(controller_api, secret, proxy_addr, clash_ua),
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
