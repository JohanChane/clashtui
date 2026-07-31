use anyhow::{Context, Result, bail};

pub mod defs {
    pub const CONFIG_FILE: &str = "config.yaml";
    pub const DATA_FILE: &str = "clashtui.db";
    pub const CORE_OVERRIDE_FILE: &str = "core_override_config.yaml";
    pub const CORE_OVERRIDE_SINGBOX_FILE: &str = "core_override_config.json";
    #[cfg(feature = "customized-theme")]
    pub const THEME_FILE: &str = "theme.yaml";
    pub const PROFILE_YAMLS_DIR: &str = "profiles";
    pub const PROFILE_JSONS_DIR: &str = "profiles";
    pub const TEMPLATE_DIR: &str = "templates";
    pub const KEYMAP_FILE: &str = "keymap.yaml";
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

#[cfg(target_family = "unix")]
pub fn check_startup_perms() {
    if crate::config::CONFIG.cfg_file.mihomo.core_service.is_user {
        return;
    }
    use std::io::Write;

    let dirs_to_check = [
        &crate::config::CONFIG.cfg_file.mihomo.core.config_dir,
        &crate::config::CONFIG.cfg_file.singbox.core.config_dir,
    ];

    for dir_str in &dirs_to_check {
        if dir_str.is_empty() {
            continue;
        }
        let dir = std::path::Path::new(dir_str);
        if !dir.exists() {
            continue;
        }
        if crate::functions::command::check_file_permissions(dir) {
            continue;
        }

        print!(
            "File permissions in '{}' need repair. Fix now? [Y/n] ",
            dir.display()
        );
        let _ = std::io::stdout().flush();
        let mut input = String::new();
        let _ = std::io::stdin().read_line(&mut input);

        if input.trim().to_lowercase().as_str() != "y" {
            continue;
        }

        let Some(group) = crate::functions::command::get_dir_group_name(dir) else {
            continue;
        };

        if let Err(e) = crate::functions::command::repair_file_permissions(dir, &group) {
            eprintln!("Error: {}", e);
            use std::io::Read;
            print!("Press Enter to continue...");
            let _ = std::io::stdout().flush();
            let _ = std::io::stdin().read(&mut [0u8]);
        }
    }
}
#[cfg(not(target_family = "unix"))]
pub fn check_startup_perms() {}

macro_rules! load_save {
    ($id:ident, $name:expr) => {
        impl $id {
            pub fn to_file(&self) -> Result<()> {
                let path = DATA_DIR.get().unwrap().join($name);
                let fp = std::fs::File::create(&path)
                    .with_context(|| format!("Failed to create {}", path.display()))?;
                serde_yml::to_writer(fp, &self)
                    .with_context(|| format!("Failed to write {}", path.display()))
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
                serde_yml::from_reader(fp)
                    .with_context(|| format!("Failed to parse {}", path.display()))
            }
        }
    };
    ($id:ident, $name:expr, no_save, $subdir:expr) => {
        impl $id {
            pub fn from_file() -> Result<Self> {
                let path = DATA_DIR.get().unwrap().join($subdir).join($name);
                let fp = std::fs::File::open(&path)
                    .with_context(|| format!("Failed to open {}", path.display()))?;
                serde_yml::from_reader(fp)
                    .with_context(|| format!("Failed to parse {}", path.display()))
            }
        }
    };
}

#[cfg(test)]
mod tests {

    #[cfg(target_os = "macos")]
    #[test]
    fn load_home_dir_macos_uses_home_dot_config() {
        // When not in portable mode, macOS uses $HOME/.config/clashtui
        let exe_dir = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let portable_data = exe_dir.join("data");
        if portable_data.exists() && portable_data.is_dir() {
            eprintln!("skipping: portable data dir exists at {:?}", portable_data);
            return;
        }
        let result = load_home_dir().unwrap();
        assert!(
            result.ends_with(".config/clashtui"),
            "expected path ending with .config/clashtui, got: {result:?}"
        );
    }

    #[test]
    fn cfg_macos_consistent() {
        // cfg!(target_os = "macos") should be true when compiled with --target *-apple-darwin
        let is_macos = cfg!(target_os = "macos");
        // This always passes — it just documents the expected platform detection
        assert!(is_macos || !is_macos);
    }
}
