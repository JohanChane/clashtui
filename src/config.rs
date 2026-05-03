//! under the data folder:
//! * [`BasicInfo`] basic_clash_config.yaml
//! * [`ProfileManager`] clashtui.db
//! * [`log`] clashtui.log
//! * [`ConfigFile`] config.yaml
//! * `Folder` profiles/
//! * `Folder` templates/

use anyhow::{Context, Result, ensure};
use core::*;
use database::*;
use std::{
    path::PathBuf,
    sync::{Mutex, OnceLock},
};
use util::*;

mod core;
#[macro_use]
mod util;
pub mod database;
#[cfg(feature = "migration_v0_2_3")]
pub mod v0_2_3;

/// Load using [init]
pub const CONFIG: Wrapper = Wrapper;

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
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
}

impl Config {
    fn load() -> Result<Self> {
        let cfg_file = ConfigFile::from_file()?;
        let basic_info = BasicInfo::from_file()?;
        let data = ProfileManager::from_file()?.into();
        Ok(Self {
            cfg_file,
            data,
            external_controller: basic_info.get_external_controller(),
            proxy_addr: basic_info
                .get_proxy_addr()
                .context("Failed to determine proxy port")?,
            secret: basic_info.secret,
            global_ua: basic_info.global_ua,
        })
    }
    pub fn save(&self) -> Result<()> {
        self.data.lock().unwrap().to_file()
    }
}

pub fn init(base_path: Option<PathBuf>) -> Result<()> {
    let path = {
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

    if DATA_DIR.set(path).is_err() || _CONFIG.set(Config::load()?).is_err() {
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

    fs::create_dir_all(path)?;

    fs::write(path.join(defs::BASIC_FILE), BasicInfo::DEFAULT)?;
    ConfigFile::default().to_file()?;
    ProfileManager::default().to_file()?;

    fs::create_dir(path.join(defs::TEMPLATE_DIR))?;
    fs::create_dir(path.join(defs::PROFILE_DIR))?;

    Ok(())
}

#[cfg(feature = "customized-theme")]
pub fn theme_path() -> PathBuf {
    DATA_DIR.get().unwrap().join(defs::TEMPLATE_DIR)
}
pub fn profile_path() -> PathBuf {
    DATA_DIR.get().unwrap().join(defs::PROFILE_DIR)
}
pub fn template_path() -> PathBuf {
    DATA_DIR.get().unwrap().join(defs::TEMPLATE_DIR)
}
pub fn load_basic() -> anyhow::Result<serde_yml::Mapping> {
    let fp = std::fs::File::create(DATA_DIR.get().unwrap().join(defs::BASIC_FILE))?;
    serde_yml::from_reader(fp).map_err(|e| e.into())
}
pub fn keymap_path() -> PathBuf {
    DATA_DIR.get().unwrap().join(defs::KEYMAP_FILE)
}

load_save!(BasicInfo, defs::BASIC_FILE, no_save);
load_save!(ConfigFile, defs::CONFIG_FILE);
load_save!(ProfileManager, defs::DATA_FILE);
