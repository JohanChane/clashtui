pub mod config;
pub mod ipc;
mod macros;
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
                            log::error!("Cannot locate absolute path:{e}");
                            log::error!("Update profile may not work");
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

pub fn setup_logging(level: u8) {
    let log_path = consts::LOG_PATH.as_path();
    #[cfg(debug_assertions)]
    let _ = std::fs::remove_file(log_path); // auto rm old log for debug
    let log_file = std::fs::File::create(log_path).unwrap();
    let flag = if log_file.metadata().is_ok_and(|m| m.len() > 1024 * 1024) {
        let _ = std::fs::remove_file(log_path);
        true
    } else {
        false
    };
    let level = level + if cfg!(debug_assertions) { 4 } else { 2 };
    let log_level = log::LevelFilter::iter()
        .nth(level as usize)
        .unwrap_or(log::LevelFilter::max());

    env_logger::builder()
        .filter_level(log_level)
        .format_timestamp_micros()
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .init();

    log::info!("{}", "-".repeat(20));
    log::trace!("Start Log, level: {}", log_level);
    if flag {
        log::info!("Log file too large, cleared")
    }
}
