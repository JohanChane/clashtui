#![warn(clippy::all)]
#[cfg(feature = "tui")]
mod tui;
mod utils;

use crate::utils::{Flag, Flags};

pub const VERSION: &str = concat!(env!("CLASHTUI_VERSION"));

/// Mihomo (Clash.Meta) TUI Client
///
/// A tui tool for mihomo
#[derive(argh::FromArgs)]
struct CliEnv {
    /// don't show UI but only update all profiles
    #[argh(switch, short = 'u')]
    update_all_profiles: bool,
    /// print version information and exit
    #[argh(switch, short = 'v')]
    version: bool,
}

fn main() {
    let CliEnv {
        update_all_profiles,
        version,
    } = argh::from_env();
    if version {
        println!("{VERSION}");
    } else {
        let mut flags = Flags::empty();
        if update_all_profiles {
            flags.insert(utils::Flag::UpdateOnly);
        };
        if let Err(e) = run(flags) {
            eprintln!("{e}");
            std::process::exit(-1)
        }
    }
    std::process::exit(0);
}
pub fn run(mut flags: Flags<Flag>) -> std::io::Result<()> {
    let config_dir = load_app_dir(&mut flags);

    setup_logging(config_dir.join("clashtui.log").to_str().unwrap());
    log::debug!("Current flags: {:?}", flags);
    let (backend, err_track) =
        utils::ClashBackend::new(&config_dir, !flags.contains(Flag::FirstInit));

    if flags.contains(Flag::UpdateOnly) {
        log::info!("Cron Mode!");
        backend
            .get_profile_names()
            .unwrap()
            .into_iter()
            .inspect(|s| println!("\nProfile: {s}"))
            .filter_map(|v| {
                backend
                    .update_profile(&v, false)
                    .map_err(|e| println!("- Error! {e}"))
                    .ok()
            })
            .flatten()
            .for_each(|s| println!("- {s}"));

        return Ok(());
    }
    // Finish cron
    else {
        #[cfg(feature = "tui")]
        {
            let mut app = tui::App::new(backend);
            use ui::setup::*;
            // setup terminal
            setup()?;
            // create app and run it
            tui::run_app(&mut app, err_track, flags)?;
            // restore terminal
            restore()?;

            app.save(config_dir.join("config.yaml").to_str().unwrap())?;
        }
        #[cfg(not(feature = "tui"))]
        {
            eprintln!("Not enable tui feature, No ui set!")
        }
    }
    Ok(())
}

fn load_app_dir(flags: &mut Flags<Flag>) -> std::path::PathBuf {
    let clashtui_config_dir = {
        use std::{env, path::PathBuf};
        let exe_dir = env::current_exe().unwrap().parent().unwrap().to_path_buf();
        let data_dir = exe_dir.join("data");
        if data_dir.exists() && data_dir.is_dir() {
            // portable mode
            flags.insert(Flag::PortableMode);
            data_dir
        } else {
            #[cfg(target_os = "linux")]
            let clashtui_config_dir_str = env::var("XDG_CONFIG_HOME")
                .or_else(|_| env::var("HOME").map(|home| format!("{}/.config/clashtui", home)))
                .unwrap();
            #[cfg(target_os = "windows")]
            let clashtui_config_dir_str = env::var("APPDATA")
                .map(|appdata| format!("{}/clashtui", appdata))
                .unwrap();
            PathBuf::from(&clashtui_config_dir_str)
        }
    };

    if !clashtui_config_dir.join("config.yaml").exists() {
        const DEFAULT_BASIC_CLASH_CFG_CONTENT: &str = r#"mixed-port: 7890
        mode: rule
        log-level: info
        external-controller: 127.0.0.1:9090"#;
        flags.insert(Flag::FirstInit);
        if let Err(err) =
            crate::utils::init_config(&clashtui_config_dir, DEFAULT_BASIC_CLASH_CFG_CONTENT)
        {
            flags.insert(Flag::ErrorDuringInit);
            log::error!("{}", err);
        }
    }
    clashtui_config_dir
}
fn setup_logging(log_path: &str) {
    use log4rs::append::file::FileAppender;
    use log4rs::config::{Appender, Config, Root};
    use log4rs::encode::pattern::PatternEncoder;
    #[cfg(debug_assertions)]
    let _ = std::fs::remove_file(log_path); // auto rm old log for debug
    let mut flag = false;
    if let Ok(m) = std::fs::File::open(log_path).and_then(|f| f.metadata()) {
        if m.len() > 1024 * 1024 {
            let _ = std::fs::remove_file(log_path);
            flag = true
        };
    }
    // No need to change. This is set to auto switch to Info level when build release
    #[allow(unused_variables)]
    let log_level = log::LevelFilter::Info;
    #[cfg(debug_assertions)]
    let log_level = log::LevelFilter::Debug;
    let file_appender = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("[{l}] {t} - {m}{n}")))
        .build(log_path)
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("file", Box::new(file_appender)))
        .build(Root::builder().appender("file").build(log_level))
        .unwrap();

    log4rs::init_config(config).unwrap();
    if flag {
        log::info!("Log file too large, clear")
    }
    log::info!("Start Log, level: {}", log_level);
}
