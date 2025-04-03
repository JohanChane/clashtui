pub const FULL_VERSION: &str = concat!(env!("CLASHTUI_VERSION"));
pub const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PKG_NAME: &str = env!("CARGO_PKG_NAME");

pub const ROOT_WARNING: &str = "Running as ROOT -- use at your own risk";

const CONFIG_FILE: &str = "config.yaml";
const DATA_FILE: &str = "clashtui.db";
const BASIC_FILE: &str = "basic_clash_config.yaml";
const LOG_FILE: &str = "clashtui.log";
#[cfg(feature = "customized-theme")]
const THEME_FILE: &str = "theme.yaml";
const PROFILE_DIR: &str = "profiles";
const TEMPLATE_DIR: &str = "templates";

pub const MAX_SUPPORTED_TEMPLATE_VERSION: u64 = 1;

use crate::DataDir;
type Lock<T> = std::sync::LazyLock<T>;
type Path = Lock<std::path::PathBuf>;

pub static CONFIG_PATH: Path = Lock::new(|| DataDir::get().join(CONFIG_FILE));
pub static DATA_PATH: Path = Lock::new(|| DataDir::get().join(DATA_FILE));
pub static BASIC_PATH: Path = Lock::new(|| DataDir::get().join(BASIC_FILE));
pub static LOG_PATH: Path = Lock::new(|| DataDir::get().join(LOG_FILE));
#[cfg(feature = "customized-theme")]
pub static THEME_PATH: Path = Lock::new(|| DataDir::get().join(THEME_FILE));
pub static PROFILE_PATH: Path = Lock::new(|| DataDir::get().join(PROFILE_DIR));
pub static TEMPLATE_PATH: Path = Lock::new(|| DataDir::get().join(TEMPLATE_DIR));

#[cfg(feature = "tui")]
pub mod err {
    pub const BACKEND_RX: &str = "backend rx dropped before STOP signal";
    pub const BACKEND_TX: &str = "backend tx dropped before STOP signal";
    pub const APP_RX: &str = "app rx dropped before STOP signal";
    pub const APP_TX: &str = "app tx dropped before STOP signal";
}
