//! under the data folder:
//! * [`BasicInfo`] mihomo/basic_clash_config.yaml
//! * [`ProfileManager`] clashtui.db
//! * [`log`] clashtui.log
//! * [`ConfigFile`] config.yaml
//! * `Folder` mihomo/profile_yamls/
//! * `Folder` mihomo/templates/
//! * `Folder` sing-box/profile_jsons/
//! * `Folder` sing-box/templates/

use anyhow::{Context, Result, ensure};
use core::*;
use database::*;
use std::{
    path::PathBuf,
    sync::{Mutex, OnceLock},
};
use util::*;

mod core;
pub use core::CoreType;
#[macro_use]
mod util;
pub mod database;
#[cfg(feature = "migration_v0_2_3")]
pub mod v0_2_3;

/// Load using [init]
pub const CONFIG: Wrapper = Wrapper;

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
static CONFIG_ROOT: OnceLock<PathBuf> = OnceLock::new();
static _CONFIG: OnceLock<Config> = OnceLock::new();

/// Wrapper around [Config], only propose is be deref-ed as [Config]
///
/// Do Not use it directly
pub struct Wrapper;

impl std::ops::Deref for Wrapper {
    type Target = Config;

    fn deref(&self) -> &Self::Target {
        _CONFIG.get().expect("uninited")
    }
}

/// Do Not use it directly, use [CONFIG] instead
pub struct Config {
    pub cfg_file: ConfigFile,
    pub data: Mutex<ProfileManager>,
    pub external_controller: String,
    pub proxy_addr: String,
    pub secret: Option<String>,
    pub global_ua: Option<String>,
    pub singbox_external_controller: String,
    pub singbox_secret: Option<String>,
}

impl Config {
    fn load() -> Result<Self> {
        let mut cfg_file = ConfigFile::from_file()?;
        let basic_info = BasicInfo::from_file()?;
        let mut data: ProfileManager = ProfileManager::from_file()?;
        // migrate File → Template for mihomo profiles with clashtui marker
        if data.migrate_file_to_template(&profile_yamls_path()) {
            let _ = data.to_file();
        }
        let data: Mutex<ProfileManager> = data.into();
        cfg_file.core_type = data.lock().unwrap().core_type;
        if !cfg_file.basic.clash_config_path.is_empty() {
            cfg_file.basic.clash_config_path = std::path::absolute(
                std::path::PathBuf::from(&cfg_file.basic.clash_config_path),
            )
            .context("Failed to resolve clash_config_path")?
            .display()
            .to_string();
        }
        if !cfg_file.basic.clash_config_dir.is_empty() {
            cfg_file.basic.clash_config_dir = std::path::absolute(
                std::path::PathBuf::from(&cfg_file.basic.clash_config_dir),
            )
            .context("Failed to resolve clash_config_dir")?
            .display()
            .to_string();
        }
        let (singbox_controller, singbox_secret) = {
            let mut secret = None;
            let controller = load_basic_singbox()
                .ok()
                .and_then(|v| {
                    let controller = v
                        .get("experimental")?
                        .get("clash_api")?
                        .get("external_controller")?
                        .as_str()?
                        .to_owned();
                    secret = v
                        .get("experimental")
                        .and_then(|e| e.get("clash_api"))
                        .and_then(|c| c.get("secret"))
                        .and_then(|s| s.as_str())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_owned());
                    if let Some(stripped) = controller.strip_prefix("http://") {
                        Some(stripped.to_owned())
                    } else {
                        Some(controller)
                    }
                })
                .unwrap_or_else(|| "127.0.0.1:9090".to_owned());
            let url = if controller.starts_with("http") {
                controller
            } else {
                format!("http://{controller}")
            };
            (url, secret)
        };
        Ok(Self {
            cfg_file,
            data,
            external_controller: basic_info.get_external_controller(),
            proxy_addr: basic_info
                .get_proxy_addr()
                .context("Failed to determine proxy port")?,
            secret: basic_info.secret,
            global_ua: basic_info.global_ua,
            singbox_external_controller: singbox_controller,
            singbox_secret,
        })
    }
    pub fn save(&self) -> Result<()> {
        self.data.lock().unwrap().to_file()
    }
    pub fn controller_for_core(&self) -> &str {
        match self.cfg_file.core_type {
            CoreType::Mihomo => &self.external_controller,
            CoreType::Singbox => &self.singbox_external_controller,
        }
    }
    pub fn secret_for_core(&self) -> Option<&str> {
        match self.cfg_file.core_type {
            CoreType::Mihomo => self.secret.as_deref(),
            CoreType::Singbox => self.singbox_secret.as_deref(),
        }
    }
}

pub fn init(base_path: Option<PathBuf>) -> Result<()> {
    let config_root = {
        let path = if let Some(path) = base_path {
            path.to_path_buf()
        } else {
            load_home_dir()?
        };
        ensure!(path.exists(), "{} does NOT exists", path.display());
        ensure!(path.is_dir(), "{} is not a dir", path.display());
        ensure!(
            path.read_dir().is_ok_and(|dir| dir.count() != 0),
            "{} is an empty dir",
            path.display()
        );

        let path = path
            .canonicalize()
            .context(format!("Failed to canonicalize path: {}", path.display()))?;
        std::path::absolute(&path).context(format!("{} is not an absolute path", path.display()))?
    };

    std::fs::create_dir_all(config_root.join("mihomo"))
        .context("Failed to create mihomo data directory")?;
    std::fs::create_dir_all(config_root.join("sing-box"))
        .context("Failed to create sing-box data directory")?;

    CONFIG_ROOT.set(config_root.clone()).ok();
    if DATA_DIR.set(config_root).is_err() || _CONFIG.set(Config::load()?).is_err() {
        unreachable!("init twice")
    }

    Ok(())
}

pub fn init_config() -> Result<()> {
    use std::fs;

    let path = match DATA_DIR.get() {
        Some(path) => path,
        None => unreachable!(),
    };
    let mihomo = path.join("mihomo");
    let singbox = path.join("sing-box");

    fs::create_dir_all(&mihomo)?;
    fs::create_dir_all(&singbox)?;

    fs::write(mihomo.join(defs::BASIC_FILE), BasicInfo::DEFAULT)?;
    fs::write(
        singbox.join(defs::BASIC_SINGBOX_FILE),
        DEFAULT_SINGBOX_BASIC_CONFIG,
    )?;
    ConfigFile::default().to_file()?;
    ProfileManager::default().to_file()?;

    fs::create_dir(mihomo.join(defs::TEMPLATE_DIR))?;
    fs::create_dir(mihomo.join(defs::PROFILE_YAMLS_DIR))?;
    fs::create_dir(singbox.join(defs::TEMPLATE_DIR))?;
    fs::create_dir(singbox.join(defs::PROFILE_JSONS_DIR))?;

    Ok(())
}

#[cfg(feature = "customized-theme")]
pub fn theme_path() -> PathBuf {
    DATA_DIR.get().unwrap().join(defs::THEME_FILE)
}
fn mihomo_dir() -> PathBuf {
    DATA_DIR.get().unwrap().join("mihomo")
}
fn singbox_dir() -> PathBuf {
    DATA_DIR.get().unwrap().join("sing-box")
}
pub fn config_dir_path() -> PathBuf {
    DATA_DIR.get().unwrap().clone()
}
pub fn config_root_path() -> PathBuf {
    CONFIG_ROOT.get().unwrap().clone()
}
pub fn template_path() -> PathBuf {
    mihomo_dir().join(defs::TEMPLATE_DIR)
}
pub fn singbox_template_path() -> PathBuf {
    singbox_dir().join(defs::TEMPLATE_DIR)
}
pub fn profile_yamls_path() -> PathBuf {
    mihomo_dir().join(defs::PROFILE_YAMLS_DIR)
}
pub fn profile_jsons_path() -> PathBuf {
    singbox_dir().join(defs::PROFILE_JSONS_DIR)
}
pub fn provider_cache_path() -> PathBuf {
    mihomo_dir().join(defs::PROVIDER_CACHE_DIR)
}
pub fn template_proxy_providers_path() -> PathBuf {
    mihomo_dir().join(defs::TEMPLATE_DIR).join("template_proxy_providers")
}
pub fn load_basic() -> anyhow::Result<serde_yml::Mapping> {
    let fp = std::fs::File::open(mihomo_dir().join(defs::BASIC_FILE))?;
    serde_yml::from_reader(fp).map_err(|e| e.into())
}
pub fn load_basic_singbox() -> anyhow::Result<serde_json::Value> {
    let fp = std::fs::File::open(singbox_dir().join(defs::BASIC_SINGBOX_FILE))?;
    serde_json::from_reader(fp).map_err(|e| e.into())
}
pub const DEFAULT_SINGBOX_BASIC_CONFIG: &str = r#"{
  "experimental": {
    "clash_api": {
      "external_controller": "127.0.0.1:9090",
      "secret": ""
    }
  },
  "inbounds": [
    {
      "type": "mixed",
      "tag": "mixed-in",
      "listen": "::",
      "listen_port": 7890
    }
  ],
  "log": {
    "level": "info"
  }
}"#;
pub fn keymap_path() -> PathBuf {
    DATA_DIR.get().unwrap().join(defs::KEYMAP_FILE)
}

load_save!(BasicInfo, defs::BASIC_FILE, no_save, "mihomo");
load_save!(ConfigFile, defs::CONFIG_FILE);
load_save!(ProfileManager, defs::DATA_FILE);
