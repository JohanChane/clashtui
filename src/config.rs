//! under the data folder:
//! * [`BasicInfo`] mihomo/core_override_config.yaml
//! * [`ProfileManager`] clashtui.db
//! * [`log`] clashtui.log
//! * [`ConfigFile`] config.yaml
//! * `Folder` mihomo/profiles/
//! * `Folder` mihomo/templates/
//! * `Folder` sing-box/profiles/
//! * `Folder` sing-box/templates/
//! * `Folder` sing-box/proxy-providers/

use anyhow::{Context, Result, ensure};
use core::*;
use database::*;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    sync::{Mutex, OnceLock},
};
use util::*;

mod core;
pub use core::{CoreType, ServiceController};
#[macro_use]
mod util;
pub mod database;
#[cfg(feature = "migration_v0_3_0")]
pub mod v0_3_0;

/// Load using [init]
pub const CONFIG: Wrapper = Wrapper;

static CORE_MISMATCH: AtomicBool = AtomicBool::new(false);

/// Set when StatusTab detects the API is serving data from a different core
/// than the configured one.
pub fn set_core_mismatch(mismatch: bool) {
    CORE_MISMATCH.store(mismatch, Ordering::Release);
}

/// True when the running core does not match the configured core type.
/// Tabs should skip displaying API data when this returns true.
pub fn is_core_mismatch() -> bool {
    CORE_MISMATCH.load(Ordering::Acquire)
}

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
        // Flush pending legacy Template migrations: write proxy_provider_groups from
        // old database entries into the corresponding template files.
        {
            let mut queue = database::PENDING_TEMPLATE_MIGRATIONS.lock().unwrap();
            for (template_name, groups) in queue.drain(..) {
                let tpl_path = template_path().join(&template_name);
                if tpl_path.exists() {
                    // Only write if the template file doesn't already have clashtui.proxy_provider_groups
                    let existing =
                        crate::functions::file::template::read_template_ppg(&template_name)
                            .unwrap_or_default();
                    if existing.is_empty() {
                        if let Err(e) = crate::functions::file::template::write_template_ppg(
                            &template_name,
                            &groups,
                        ) {
                            log::error!(
                                "Failed to migrate proxy_provider_groups to template '{template_name}': {e}"
                            );
                        } else {
                            log::info!(
                                "Migrated proxy_provider_groups from database to template '{template_name}'"
                            );
                        }
                    } else {
                        log::info!(
                            "Template '{template_name}' already has proxy_provider_groups, skipping migration"
                        );
                    }
                } else {
                    log::warn!(
                        "Template file '{template_name}' not found for migration — groups will be dropped on next save"
                    );
                }
            }
        }
        let data: Mutex<ProfileManager> = data.into();
        if !cfg_file.mihomo.core.config_path.is_empty() {
            cfg_file.mihomo.core.config_path =
                std::path::absolute(std::path::PathBuf::from(&cfg_file.mihomo.core.config_path))
                    .context("Failed to resolve mihomo config_path")?
                    .display()
                    .to_string();
        }
        if !cfg_file.mihomo.core.config_dir.is_empty() {
            cfg_file.mihomo.core.config_dir =
                std::path::absolute(std::path::PathBuf::from(&cfg_file.mihomo.core.config_dir))
                    .context("Failed to resolve mihomo config_dir")?
                    .display()
                    .to_string();
        }
        if !cfg_file.singbox.core.config_dir.is_empty() {
            cfg_file.singbox.core.config_dir =
                std::path::absolute(std::path::PathBuf::from(&cfg_file.singbox.core.config_dir))
                    .context("Failed to resolve singbox config_dir")?
                    .display()
                    .to_string();
        }
        if !cfg_file.singbox.core.config_path.is_empty() {
            cfg_file.singbox.core.config_path =
                std::path::absolute(std::path::PathBuf::from(&cfg_file.singbox.core.config_path))
                    .context("Failed to resolve singbox config_path")?
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
    pub fn core_type(&self) -> CoreType {
        self.data.lock().unwrap().core_type
    }
    pub fn save(&self) -> Result<()> {
        self.data.lock().unwrap().to_file()
    }
    pub fn controller_for_core(&self) -> &str {
        match self.data.lock().unwrap().core_type {
            CoreType::Mihomo => &self.external_controller,
            CoreType::Singbox => &self.singbox_external_controller,
        }
    }
    pub fn secret_for_core(&self) -> Option<&str> {
        match self.data.lock().unwrap().core_type {
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
        if !path.exists() {
            std::fs::create_dir_all(&path).with_context(|| {
                format!("Failed to create config directory: {}", path.display())
            })?;
        }
        ensure!(path.is_dir(), "{} is not a dir", path.display());

        let path = path
            .canonicalize()
            .context(format!("Failed to canonicalize path: {}", path.display()))?;
        std::path::absolute(&path).context(format!("{} is not an absolute path", path.display()))?
    };

    let is_first_run = !config_root.join(defs::CONFIG_FILE).exists();

    std::fs::create_dir_all(config_root.join("mihomo"))
        .context("Failed to create mihomo data directory")?;
    std::fs::create_dir_all(config_root.join("sing-box"))
        .context("Failed to create sing-box data directory")?;

    CONFIG_ROOT.set(config_root.clone()).ok();
    if DATA_DIR.set(config_root).is_err() {
        unreachable!("init twice")
    }

    if is_first_run {
        init_config()?;
    }

    // Fill in files that may be missing from old install scripts or partial setups
    {
        use std::fs;
        let path = DATA_DIR.get().unwrap();

        let mihomo_override = path.join("mihomo").join(defs::CORE_OVERRIDE_FILE);
        if !mihomo_override.exists() {
            fs::write(&mihomo_override, BasicInfo::DEFAULT)
                .with_context(|| format!("Failed to write {}", mihomo_override.display()))?;
        }
        let singbox_override = path.join("sing-box").join(defs::CORE_OVERRIDE_SINGBOX_FILE);
        if !singbox_override.exists() {
            fs::write(&singbox_override, DEFAULT_SINGBOX_BASIC_CONFIG)
                .with_context(|| format!("Failed to write {}", singbox_override.display()))?;
        }
        let db = path.join(defs::DATA_FILE);
        if !db.exists() {
            ProfileManager::default().to_file()?;
        }
    }

    if _CONFIG.set(Config::load()?).is_err() {
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

    fs::write(mihomo.join(defs::CORE_OVERRIDE_FILE), BasicInfo::DEFAULT)?;
    fs::write(
        singbox.join(defs::CORE_OVERRIDE_SINGBOX_FILE),
        DEFAULT_SINGBOX_BASIC_CONFIG,
    )?;
    ConfigFile::default().to_file()?;
    ProfileManager::default().to_file()?;

    fs::create_dir(mihomo.join(defs::TEMPLATE_DIR))?;
    fs::create_dir(mihomo.join(defs::PROFILE_YAMLS_DIR))?;
    fs::create_dir(singbox.join(defs::TEMPLATE_DIR))?;
    fs::create_dir(singbox.join(defs::PROFILE_JSONS_DIR))?;
    fs::create_dir(singbox.join(defs::PROXY_PROVIDERS_DIR))?;

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
pub fn singbox_proxy_providers_path() -> PathBuf {
    singbox_dir().join(defs::PROXY_PROVIDERS_DIR)
}
pub fn singbox_core_override_path() -> PathBuf {
    singbox_dir().join(defs::CORE_OVERRIDE_SINGBOX_FILE)
}
pub fn load_basic() -> anyhow::Result<serde_yml::Mapping> {
    let fp = std::fs::File::open(mihomo_dir().join(defs::CORE_OVERRIDE_FILE))?;
    serde_yml::from_reader(fp).map_err(|e| e.into())
}
pub fn load_basic_singbox() -> anyhow::Result<serde_json::Value> {
    let fp = std::fs::File::open(singbox_dir().join(defs::CORE_OVERRIDE_SINGBOX_FILE))?;
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
      "listen": "127.0.0.1",
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

load_save!(BasicInfo, defs::CORE_OVERRIDE_FILE, no_save, "mihomo");
load_save!(ConfigFile, defs::CONFIG_FILE);
load_save!(ProfileManager, defs::DATA_FILE);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_mismatch_flag_defaults_false() {
        assert!(!is_core_mismatch());
    }

    #[test]
    fn core_mismatch_flag_toggle() {
        set_core_mismatch(true);
        assert!(is_core_mismatch());
        set_core_mismatch(false);
        assert!(!is_core_mismatch());
    }
}
