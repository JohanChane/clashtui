macro_rules! pm {
    () => {
        crate::config::CONFIG.data.lock().unwrap()
    };
}

pub mod profile;
pub mod template;

use std::{path::PathBuf, sync::LazyLock};

static PROFILE_PATH: LazyLock<PathBuf> = LazyLock::new(crate::config::profile_path);
static TEMPLATE_PATH: LazyLock<PathBuf> = LazyLock::new(crate::config::template_path);

const MAX_SUPPORTED_TEMPLATE_VERSION: u64 = 1;
