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

pub fn setup_logging(verbose: u8) {
    use log4rs::append::file::FileAppender;
    use log4rs::config::{Appender, Config, Root};
    use log4rs::encode::pattern::PatternEncoder;

    let log_file = consts::LOG_PATH.as_path();
    #[cfg(debug_assertions)]
    let _ = std::fs::remove_file(log_file); // auto rm old log for debug
    let flag = if std::fs::File::open(log_file)
        .and_then(|f| f.metadata())
        .is_ok_and(|m| m.len() > 1024 * 1024)
    {
        let _ = std::fs::remove_file(log_file);
        true
    } else {
        false
    };
    let verbose = verbose + if cfg!(debug_assertions) { 4 } else { 2 };
    let log_level = log::LevelFilter::iter()
        .nth(verbose as usize)
        .unwrap_or(log::LevelFilter::max());

    let file_appender = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%H:%M:%S)} [{l}] {t} - {m}{n}",
        ))) // Having a timestamp would be better.
        .build(log_file)
        .expect("Err opening log file");

    let config = Config::builder()
        .appender(Appender::builder().build("file", Box::new(file_appender)))
        .build(Root::builder().appender("file").build(log_level))
        .expect("Err building log config");

    log4rs::init_config(config).expect("Err initing log service");

    log::info!("{}", "-".repeat(20));
    log::info!("Start Log, level: {}", log_level);
    if flag {
        log::info!("Log file too large, cleared")
    }
}
