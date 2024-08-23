pub(crate) const VERSION: &str = concat!(env!("CLASHTUI_VERSION"));

pub(crate) const ROOT_WARNING: &str = "Running as ROOT -- use at your own risk";

pub(crate) const CONFIG_FILE: &str = "config.yaml";
pub(crate) const DATA_FILE: &str = "clashtui.db";
pub(crate) const BASIC_FILE: &str = "basic_clash_config.yaml";
pub(crate) const LOG_FILE: &str = "clashtui.log";
pub(crate) const _TMP_FILE: &str = "/tmp/clashtui_mihomo_config_file.tmp";

pub(crate) const LOCALHOST: &str = "127.0.0.1";

pub mod err {
    pub const BACKEND_RX: &str = "backend rx dropped before STOP singal";
    pub const BACKEND_TX: &str = "backend tx dropped before STOP singal";
    pub const APP_RX: &str = "app rx dropped before STOP singal";
    pub const APP_TX: &str = "app tx dropped before STOP singal";
}
