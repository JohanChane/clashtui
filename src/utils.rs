pub mod config;
pub mod ipc;
pub mod logging;
pub mod self_update;

pub(crate) mod consts;
pub(crate) use config::BuildConfig;

mod data_dir {
    static DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    use crate::DataDir;

    impl DataDir {
        fn check(path: std::path::PathBuf) -> Option<std::path::PathBuf> {
            if path.exists() && path.is_dir() {
                if let Ok(path) = path.canonicalize() {
                    let path = match std::path::absolute(&path) {
                        Ok(dir) => dir,
                        Err(e) => {
                            eprintln!("Cannot locate absolute path:{e}");
                            eprintln!("Update profile may not work");
                            path
                        }
                    };
                    return Some(path);
                }
            }
            None
        }
        pub fn set(value: std::path::PathBuf) {
            if let Some(path) = Self::check(value) {
                let _ = DIR.set(path);
            }
        }
        pub fn get() -> &'static std::path::PathBuf {
            DIR.get_or_init(|| {
                // try get from prefix
                if let Some(data_dir) = std::env::var_os("CLASHTUI_CONFIG_DIR")
                    .map(std::path::PathBuf::from)
                    .and_then(Self::check)
                {
                    data_dir
                } else {
                    // search 'data' or '~/.config/clashtui'
                    load_home_dir()
                }
            })
        }
    }

    fn load_home_dir() -> std::path::PathBuf {
        use std::{env, path};
        let data_dir = env::current_exe()
            .expect("Err loading exe_file_path")
            .parent()
            .expect("Err finding exe_dir")
            .join("data");
        if data_dir.exists() && data_dir.is_dir() {
            // portable mode
            data_dir
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
                unimplemented!("Not supported platform")
            }
            .map(|c| c.join("clashtui"))
            .expect("failed to load home dir")
        }
    }
}
