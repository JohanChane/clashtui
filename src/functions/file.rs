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
static TEMPLATE_PATH: LazyLock<PathBuf> = LazyLock::new(crate::config::template_path);

const MAX_SUPPORTED_TEMPLATE_VERSION: u64 = 1;
