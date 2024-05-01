#![warn(clippy::all)]
mod commands;
#[cfg(feature = "tui")]
mod tui;
mod utils;

use utils::{
    VERSION, {init_config, ClashBackend, Flag, Flags},
};

fn main() {
    if let Ok(infos) = commands::parse_args() {
        if infos.flags.contains(commands::Flag::Version) {
            println!("{VERSION}");
        } else {
            let mut flags = Flags::empty();
            if !infos.flags.contains(commands::Flag::Tui) {
                flags.insert(Flag::CliMode)
            }
            if let Err(e) = run(flags) {
                eprintln!("{e}");
                std::process::exit(-1)
            }
        }
        std::process::exit(0);
    }
}
pub fn run(mut flags: Flags<Flag>) -> Result<(), impl core::fmt::Display> {
    let config_dir = load_app_dir(&mut flags);

    setup_logging(config_dir.join("clashtui.log").to_str().unwrap());
    log::debug!("Current flags: {:?}", flags);
    let (backend, err_track) = ClashBackend::new(&config_dir, !flags.contains(Flag::FirstInit));

    if flags.contains(Flag::CliMode) {
        if !err_track.is_empty() {
            println!("Some err happened, you may have to fix them before this program can work as expected");
            err_track.into_iter().for_each(|e| println!("{e}"));
        }
        if let Some(r) = commands::handle_cli_args(backend) {
            match r {
                Ok(v) => println!("{v}"),
                Err(e) => eprintln!("{e}"),
            }
        } else {
            return Err("para error");
        }
    } else {
        #[cfg(feature = "tui")]
        {
            let mut app = tui::App::new(backend);
            use ui::setup::*;
            // setup terminal
            setup()?;
            // create app and run it
            app.run(err_track, flags)?;
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
        if let Err(err) = init_config(&clashtui_config_dir, DEFAULT_BASIC_CLASH_CFG_CONTENT) {
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
