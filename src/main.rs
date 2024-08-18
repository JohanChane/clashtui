#![warn(clippy::all)]
mod api;
mod backend;
mod commands;
#[cfg(feature = "tui")]
mod tui;
#[cfg(feature = "tui")]
mod ui;
mod utils;

use backend::const_err::ERR_PATH_UTF_8;
use utils::{consts, init_config, ClashBackend, Flag, Flags};

fn main() {
    /*
    // Prevent create root files before fixing files permissions.
    if utils::is_clashtui_ep() {
        utils::mock_fileop_as_sudo_user();
    }
    // To allow the mihomo process to read and write files created by clashtui in clash_cfg_dir, set the umask to 0o002.
    sys::stat::umask(sys::stat::Mode::from_bits_truncate(0o002));
     */
    if let Ok(infos) = commands::parse_args() {
        let mut flags = Flags::empty();
        let config_dir = load_app_dir(&mut flags);

        setup_logging(
            config_dir
                .join(consts::LOG_NAME)
                .to_str()
                .expect(ERR_PATH_UTF_8),
        );
        log::debug!("Current flags: {:?}", flags);
        let (backend, err_track) = ClashBackend::new(&config_dir, !flags.contains(Flag::FirstInit));
        if let Some(command) = infos {
            if !err_track.is_empty() {
                println!("Some err happened, you may have to fix them before this program can work as expected");
                err_track.into_iter().for_each(|e| println!("{e}"));
            }
            if is_root::is_root() {
                println!("{}", consts::ROOT_WARNING)
            }
            match commands::handle_cli(command, backend) {
                Ok(v) => {
                    println!("{v}")
                }
                Err(e) => {
                    eprintln!("{e}");
                    log::error!("Cli:{e:?}");
                    std::process::exit(-1)
                }
            }
        } else {
            #[cfg(feature = "tui")]
            if let Err(e) = run_tui(backend, err_track, flags) {
                eprintln!("{e}");
                log::error!("Tui:{e:?}");
                ui::setup::restore().expect("Err restore terminal interface");
                std::process::exit(-1)
            }
            #[cfg(not(feature = "tui"))]
            {
                eprintln!("Not enable tui feature, No ui set!");
                std::process::exit(-1)
            }
        }
        std::process::exit(0);
    }
}
#[cfg(feature = "tui")]
pub fn run_tui(
    backend: ClashBackend,
    err_track: Vec<anyhow::Error>,
    flags: Flags<Flag>,
) -> Result<(), std::io::Error> {
    let mut app = tui::App::new(backend);
    // setup terminal
    ui::setup::setup()?;
    // create app and run it
    app.run(err_track, flags)?;

    ui::setup::restore()?;

    Ok(())
}

fn load_app_dir(flags: &mut Flags<Flag>) -> std::path::PathBuf {
    let config_dir = {
        use std::{env, path::PathBuf};
        let exe_dir = env::current_exe()
            .expect("Err loading exe_file_path")
            .parent()
            .expect("Err finding exe_dir")
            .to_path_buf();
        let data_dir = exe_dir.join("data");
        if data_dir.exists() && data_dir.is_dir() {
            // portable mode
            flags.insert(Flag::PortableMode);
            data_dir
        } else {
            #[cfg(target_os = "linux")]
            let config_dir_str = env::var("XDG_CONFIG_HOME")
                .or_else(|_| env::var("HOME").map(|home| format!("{}/.config/clashtui", home)));
            #[cfg(target_os = "windows")]
            let config_dir_str = env::var("APPDATA").map(|appdata| format!("{}/clashtui", appdata));
            #[cfg(target_os = "macos")]
            let config_dir_str = env::var("HOME").map(|home| format!("{}/.config/clashtui", home));
            PathBuf::from(&config_dir_str.expect("Err loading global config dir"))
        }
    };

    if !config_dir.join("config.yaml").exists() {
        flags.insert(Flag::FirstInit);
        if let Err(e) = init_config(&config_dir) {
            eprintln!("Err during init:{e}");
            std::process::exit(-1)
        }
    }
    config_dir
}

fn setup_logging(log_path: &str) {
    use log4rs::append::file::FileAppender;
    use log4rs::config::{Appender, Config, Root};
    use log4rs::encode::pattern::PatternEncoder;
    #[cfg(debug_assertions)]
    let _ = std::fs::remove_file(log_path); // auto rm old log for debug
    let flag = if std::fs::File::open(log_path)
        .and_then(|f| f.metadata())
        .is_ok_and(|m| m.len() > 1024 * 1024)
    {
        let _ = std::fs::remove_file(log_path);
        true
    } else {
        false
    };
    // No need to change. This is set to auto switch to Info level when build release
    #[allow(unused_variables)]
    let log_level = log::LevelFilter::Info;
    #[cfg(debug_assertions)]
    let log_level = log::LevelFilter::Debug;
    let file_appender = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%H:%M:%S)} [{l}] {t} - {m}{n}",
        ))) // Having a timestamp would be better.
        .build(log_path)
        .expect("Err opening log file");

    let config = Config::builder()
        .appender(Appender::builder().build("file", Box::new(file_appender)))
        .build(Root::builder().appender("file").build(log_level))
        .expect("Err building log config");

    log4rs::init_config(config).expect("Err initing log service");

    log::info!("Start Log, level: {}", log_level);
    if flag {
        log::info!("Log file too large, clear")
    }
}
