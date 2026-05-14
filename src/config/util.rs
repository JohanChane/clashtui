use anyhow::{Context, Result, bail};

pub mod defs {
    pub const CONFIG_FILE: &str = "config.yaml";
    pub const DATA_FILE: &str = "clashtui.db";
    pub const CORE_OVERRIDE_FILE: &str = "core_override_config.yaml";
    pub const CORE_OVERRIDE_SINGBOX_FILE: &str = "core_override_config.json";
    pub const LOG_FILE: &str = "clashtui.log";
    #[cfg(feature = "customized-theme")]
    pub const THEME_FILE: &str = "theme.yaml";
    pub const PROFILE_YAMLS_DIR: &str = "profiles";
    pub const PROFILE_JSONS_DIR: &str = "profiles";
    pub const TEMPLATE_DIR: &str = "templates";
    pub const KEYMAP_FILE: &str = "keymap.yaml";
    pub const PROVIDER_CACHE_DIR: &str = "providers";
    pub const PROXY_PROVIDERS_DIR: &str = "proxy-providers";
}

pub(super) fn load_home_dir() -> Result<std::path::PathBuf> {
    use std::{env, path};
    let data_dir = env::current_exe()
        .context("Err loading exe_file_path")?
        .parent()
        .context("Err finding exe_dir")?
        .join("data");
    if data_dir.exists() && data_dir.is_dir() {
        // portable mode
        Ok(data_dir)
    } else {
        if cfg!(target_os = "linux") {
            env::var_os("XDG_CONFIG_HOME")
                .map(path::PathBuf::from)
                .or(env::var_os("HOME").map(|h| path::PathBuf::from(h).join(".config")))
        } else if cfg!(target_os = "windows") {
            env::var_os("APPDATA").map(path::PathBuf::from)
        } else if cfg!(target_os = "macos") {
            env::var_os("HOME").map(|h| path::PathBuf::from(h).join(".config"))
        } else {
            bail!("Not supported platform")
        }
        .map(|c| c.join("clashtui"))
        .context("failed to load home dir")
    }
}

macro_rules! load_save {
    ($id:ident, $name:expr) => {
        impl $id {
            pub fn to_file(&self) -> Result<()> {
                let path = DATA_DIR.get().unwrap().join($name);
                let fp = std::fs::File::create(&path)
                    .with_context(|| format!("Failed to create {}", path.display()))?;
                Ok(serde_yml::to_writer(fp, &self)
                    .with_context(|| format!("Failed to write {}", path.display()))?)
            }
        }
        load_save!($id, $name, no_save);
    };
    ($id:ident, $name:expr, no_save) => {
        impl $id {
            pub fn from_file() -> Result<Self> {
                let path = DATA_DIR.get().unwrap().join($name);
                let fp = std::fs::File::open(&path)
                    .with_context(|| format!("Failed to open {}", path.display()))?;
                Ok(serde_yml::from_reader(fp)
                    .with_context(|| format!("Failed to parse {}", path.display()))?)
            }
        }
    };
    ($id:ident, $name:expr, no_save, $subdir:expr) => {
        impl $id {
            pub fn from_file() -> Result<Self> {
                let path = DATA_DIR.get().unwrap().join($subdir).join($name);
                let fp = std::fs::File::open(&path)
                    .with_context(|| format!("Failed to open {}", path.display()))?;
                Ok(serde_yml::from_reader(fp)
                    .with_context(|| format!("Failed to parse {}", path.display()))?)
            }
        }
    };
}
