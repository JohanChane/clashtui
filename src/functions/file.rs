macro_rules! pm {
    () => {
        crate::config::CONFIG.data.lock().unwrap()
    };
}

pub mod net_resource;
pub mod profile;
pub mod template;

use std::{path::PathBuf, sync::LazyLock};

pub static PROFILE_YAMLS_PATH: LazyLock<PathBuf> = LazyLock::new(crate::config::profile_yamls_path);
pub static PROFILE_JSONS_PATH: LazyLock<PathBuf> = LazyLock::new(crate::config::profile_jsons_path);
pub(crate) static TEMPLATE_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| match crate::config::CONFIG.core_type() {
        crate::config::CoreType::Mihomo => crate::config::template_path(),
        crate::config::CoreType::Singbox => crate::config::singbox_template_path(),
    });

const MAX_SUPPORTED_TEMPLATE_VERSION: u64 = 1;
