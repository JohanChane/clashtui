use std::cell::RefCell;
use std::{
    fs::File,
    io::{Error, Read},
    path::{Path, PathBuf},
};

#[cfg(target_os = "windows")]
use encoding::all::GBK;

use super::super::lib::{ClashConfig, ClashUtil};
use super::{
    config::{ClashTuiConfig, ClashTuiConfigLoadError},
    state::_State,
    CfgOp,
};

pub struct ClashTuiUtil {
    pub clashtui_dir: PathBuf,
    pub profile_dir: PathBuf,

    clash_api: ClashUtil,
    clashtui_config: RefCell<ClashTuiConfig>,
    clash_remote_config: RefCell<Option<ClashConfig>>,
}

// Misc
impl ClashTuiUtil {
    pub fn new(
        clashtui_dir: &PathBuf,
        profile_dir: &Path,
        is_inited: bool,
    ) -> (Self, Vec<ClashTuiConfigLoadError>) {
        let ret = load_app_config(clashtui_dir, is_inited);
        let mut err_track = ret.3;
        let clash_api = ClashUtil::new(ret.1, ret.2);
        let cur_remote = match clash_api.config_get() {
            Ok(v) => v,
            Err(_) => String::new(),
        };
        let remote = ClashConfig::from_str(cur_remote.as_str());
        if remote.is_none() {
            err_track.push(ClashTuiConfigLoadError::LoadClashConfig(
                "Fail to load config from clash core. Is it Running?".to_string(),
            ));
            log::warn!("Fail to connect to clash. Is it Running?");
        }
        (
            Self {
                clashtui_dir: clashtui_dir.clone(),
                profile_dir: profile_dir.to_path_buf(),
                clash_api,
                clashtui_config: RefCell::new(ret.0),
                clash_remote_config: RefCell::new(remote),
            },
            err_track,
        )
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
// Config Related
impl ClashTuiUtil {
    pub fn get_cfg(&self, typ: CfgOp) -> String {
        let config = self.clashtui_config.borrow();
        match typ {
            CfgOp::ClashConfigDir => config.clash_cfg_dir.clone(),
            CfgOp::ClashCorePath => config.clash_core_path.clone(),
            CfgOp::ClashConfigFile => config.clash_cfg_path.clone(),
            CfgOp::ClashServiceName => config.clash_srv_name.clone(),
            CfgOp::TuiEdit => config.edit_cmd.clone(),
            CfgOp::TuiOpen => config.open_dir_cmd.clone(),
        }
    }

    pub fn update_cfg(&self, conf: &CfgOp, data: String) {
        let mut config = self.clashtui_config.borrow_mut();
        log::debug!("Updated Config: {:?}:{}", conf, data);
        match conf {
            CfgOp::ClashConfigDir => config.clash_cfg_dir = data,
            CfgOp::ClashCorePath => config.clash_core_path = data,
            CfgOp::ClashConfigFile => config.clash_cfg_path = data,
            CfgOp::ClashServiceName => config.clash_srv_name = data,
            CfgOp::TuiEdit => config.edit_cmd = data,
            CfgOp::TuiOpen => config.open_dir_cmd = data,
        };
        drop(config);
        self.save_cfg();
    }

    pub fn save_cfg(&self) {
        if let Err(x) = self
            .clashtui_config
            .borrow()
            .to_file(self.clashtui_dir.join("config.yaml").to_str().unwrap())
        {
            log::error!("Error while saving config: {}", x);
        };
    }
}
// Web
impl ClashTuiUtil {
    fn fetch_remote(&self) -> Option<reqwest::Error> {
        let cur_remote = match self.clash_api.config_get() {
            Ok(v) => v,
            Err(e) => return Some(e),
        };
        let remote = ClashConfig::from_str(cur_remote.as_str());
        *self.clash_remote_config.borrow_mut() = remote;
        None
    }

    pub fn restart_clash(&self) -> Result<String, reqwest::Error> {
        self.clash_api.restart(None)
    }

    pub fn select_profile(&self, profile_name: &String) -> Result<(), Error> {
        if let Err(err) = self.merge_profile(profile_name) {
            log::error!(
                "Failed to Merge Profile `{}`: {}",
                profile_name,
                err.to_string()
            );
            return Err(Error::new(std::io::ErrorKind::Other, err));
        };
        let body = serde_json::json!({
            "path": self.clashtui_config.borrow().clash_cfg_path.as_str(),
            "payload": ""
        })
        .to_string();
        if let Some(err) = self.clash_api.config_reload(body) {
            log::error!(
                "Failed to Patch Profile `{}`: {}",
                profile_name,
                err.to_string()
            );
            return Err(Error::new(std::io::ErrorKind::Other, err));
        };
        Ok(())
    }

    fn merge_profile(&self, profile_name: &String) -> anyhow::Result<()> {
        let basic_clash_cfg_path = self.clashtui_dir.join("basic_clash_config.yaml");
        let mut dst_parsed_yaml = parse_yaml(&basic_clash_cfg_path)?;
        let profile_yaml_path = self.get_profile_yaml_path(profile_name);
        let profile_parsed_yaml = parse_yaml(&profile_yaml_path).or_else(|e| {
            anyhow::bail!(
                "Maybe need to update first. Failed to parse {}: {}",
                profile_yaml_path.to_str().unwrap(),
                e.to_string()
            );
        })?;

        if let serde_yaml::Value::Mapping(dst_mapping) = &mut dst_parsed_yaml {
            if let serde_yaml::Value::Mapping(mapping) = &profile_parsed_yaml {
                for (key, value) in mapping.iter() {
                    if let serde_yaml::Value::String(k) = key {
                        match k.as_str() {
                            "proxy-groups" | "proxy-providers" | "proxies" | "sub-rules"
                            | "rules" | "rule-providers" => {
                                dst_mapping.insert(key.clone(), value.clone());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        let final_clash_cfg_file = File::create(&self.clashtui_config.borrow().clash_cfg_path)?;
        serde_yaml::to_writer(final_clash_cfg_file, &dst_parsed_yaml)?;

        Ok(())
    }

    pub fn dl_remote_profile(
        &self,
        url: &str,
    ) -> Result<reqwest::blocking::Response, reqwest::Error> {
        self.clash_api.mock_clash_core(url)
    }

    #[cfg(target_os = "linux")]
    pub fn update_state(&self, new_pf: Option<String>, new_mode: Option<String>) -> _State {
        let is_skip = new_pf.is_none() && new_mode.is_none();
        if let Some(v) = new_mode {
            let load = format!(r#"{{"mode": "{}"}}"#, v);
            let _ = self
                .clash_api
                .config_patch(load)
                .map_err(|e| log::error!("Patch Errr: {}", e));
        }

        let mut tuiconf = self.clashtui_config.borrow_mut();
        let pf = match new_pf {
            Some(v) => {
                tuiconf.update_profile(v.clone());
                v
            }
            None => tuiconf.current_profile.clone(),
        };

        let ver = match self.clash_api.version() {
            Ok(v) => v,
            Err(e) => {
                log::warn!("{}", e);
                "Unknown".to_string()
            }
        };

        if !is_skip {
            if let Some(e) = self.fetch_remote() {
                if !e.is_connect() {
                    log::warn!("{}", e);
                }
            }
        }
        let mode;
        let tun = match self.clash_remote_config.borrow().as_ref() {
            Some(v) => {
                mode = v.mode.to_string();
                if v.tun.enable {
                    v.tun.stack.to_string()
                } else {
                    "False".to_string()
                }
            }
            None => {
                mode = "UnKnown".to_string();
                "Unknown".to_string()
            }
        };
        _State::new(pf, mode, tun, ver)
    }
}

pub(super) fn parse_yaml(yaml_path: &Path) -> anyhow::Result<serde_yaml::Value> {
    let mut file = File::open(yaml_path)?;
    let mut yaml_content = String::new();
    file.read_to_string(&mut yaml_content)?;
    let parsed_yaml_content: serde_yaml::Value =
        serde_yaml::from_str(yaml_content.as_str()).unwrap();
    Ok(parsed_yaml_content)
}

fn load_app_config(
    clashtui_dir: &PathBuf,
    skip_init_conf: bool,
) -> (ClashTuiConfig, String, String, Vec<ClashTuiConfigLoadError>) {
    let mut err_collect = Vec::new();
    let basic_clash_config_path = Path::new(clashtui_dir).join("basic_clash_config.yaml");
    let basic_clash_config_value: serde_yaml::Value =
        match parse_yaml(basic_clash_config_path.as_path()) {
            Ok(r) => r,
            Err(_) => {
                err_collect.push(ClashTuiConfigLoadError::LoadProfileConfig(
                    "Fail to load User Defined Config\n".into(),
                ));
                serde_yaml::Value::from("")
            }
        };
    let controller_api = if let Some(controller_api) = basic_clash_config_value
        .get("external-controller")
        .and_then(|v| v.as_str())
    {
        format!("http://{}", controller_api)
    } else {
        "http://127.0.0.1:9090".to_string()
    };
    log::debug!("controller_api: {}", controller_api);

    let proxy_addr = get_proxy_addr(&basic_clash_config_value);
    log::debug!("proxy_addr: {}", proxy_addr);

    let configs = if skip_init_conf {
        let config_path = clashtui_dir.join("config.yaml");
        match ClashTuiConfig::from_file(config_path.to_str().unwrap()) {
            Ok(v) => {
                if !v.check() {
                    err_collect.push(ClashTuiConfigLoadError::LoadAppConfig(
                        "Some Key Configs are missing, or Default\n".into(),
                    ));
                    log::warn!("Empty Config?");
                    log::debug!("{:?}", v)
                };
                v
            }
            Err(e) => {
                err_collect.push(ClashTuiConfigLoadError::LoadAppConfig(
                    "Fail to load configs, using Default\n".into(),
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

    format!("http://{}:7890", host)
}
