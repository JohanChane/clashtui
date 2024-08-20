use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Basic {
    #[serde(rename = "clash_config_dir")]
    pub clash_cfg_dir: String,
    #[serde(rename = "clash_bin_path")]
    pub clash_bin_pth: String,
    #[serde(rename = "clash_config_path")]
    pub clash_cfg_pth: String,
}
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Service {
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    pub clash_srv_nam: String,
    #[cfg(target_os = "linux")]
    pub is_user: bool,
}

pub struct LibConfig {
    pub basic: Basic,
    pub service: Service,
}
