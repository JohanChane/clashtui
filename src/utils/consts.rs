pub(crate) const VERSION: &str = concat!(env!("CLASHTUI_VERSION"));

pub(crate) const ROOT_WARNING: &str = "Running as ROOT -- use at your own risk";

const CONFIG_FILE: &str = "config.yaml";
const DATA_FILE: &str = "clashtui.db";
const BASIC_FILE: &str = "basic_clash_config.yaml";
const LOG_FILE: &str = "clashtui.log";
#[cfg(feature = "customized-theme")]
const THEME_FILE: &str = "theme.yaml";
const PROFILE_DIR: &str = "profiles";
const TEMPLATE_DIR: &str = "templates";
const _TMP_FILE: &str = "/tmp/clashtui_mihomo_config_file.tmp";

pub(crate) const MAX_SUPPORTED_TEMPLATE_VERSION: u8 = 1;

use crate::DataDir;
type Lock<T> = std::sync::LazyLock<T>;
type Path = Lock<std::path::PathBuf>;

pub(crate) static CONFIG_PATH: Path = Lock::new(|| DataDir::get().join(CONFIG_FILE));
pub(crate) static DATA_PATH: Path = Lock::new(|| DataDir::get().join(DATA_FILE));
pub(crate) static BASIC_PATH: Path = Lock::new(|| DataDir::get().join(BASIC_FILE));
pub(crate) static LOG_PATH: Path = Lock::new(|| DataDir::get().join(LOG_FILE));
#[cfg(feature = "customized-theme")]
pub(crate) static THEME_PATH: Path = Lock::new(|| DataDir::get().join(THEME_FILE));
pub(crate) static PROFILE_PATH: Path = Lock::new(|| DataDir::get().join(PROFILE_DIR));
pub(crate) static TEMPLATE_PATH: Path = Lock::new(|| DataDir::get().join(TEMPLATE_DIR));
pub(crate) static _TEMP_PATH: Path = Lock::new(|| DataDir::get().join(_TMP_FILE));

pub mod err {
    pub const BACKEND_RX: &str = "backend rx dropped before STOP signal";
    pub const BACKEND_TX: &str = "backend tx dropped before STOP signal";
    pub const APP_RX: &str = "app rx dropped before STOP signal";
    pub const APP_TX: &str = "app tx dropped before STOP signal";
}
