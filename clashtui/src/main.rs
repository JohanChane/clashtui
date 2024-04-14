#![warn(clippy::all)]
mod app;
mod tui;
mod utils;

use core::time::Duration;
use nix::sys;

use crate::app::App;
use crate::utils::{Flag, Flags};

pub const VERSION: &str = concat!(env!("CLASHTUI_VERSION"));

fn main() {
    // Prevent create root files before fixing files permissions.
    if utils::is_clashtui_ep() {
        utils::mock_fileop_as_sudo_user();
    }
    // To allow the mihomo process to read and write files created by clashtui in clash_cfg_dir, set the umask to 0o002.
    sys::stat::umask(sys::stat::Mode::from_bits_truncate(0o002));

    let mut warning_list_msg = Vec::<String>::new();

    // ## Paser param
    let cli_env: app::CliEnv = argh::from_env();
    if cli_env.version {
        println!("{VERSION}");
        std::process::exit(0);
    }

    let mut flags = Flags::empty();

    // ## Setup logging as early as possible. So We can log.
    let config_dir = load_app_dir(&mut flags);
    setup_logging(config_dir.join("clashtui.log").to_str().unwrap());

    let tick_rate = 250;    // time in ms between two ticks.
    if let Err(e) = run(&mut flags, tick_rate, &config_dir, &mut warning_list_msg) {
        eprintln!("{e}");
        std::process::exit(-1)
    }

    std::process::exit(0);
}
pub fn run(flags: &mut Flags<Flag>, tick_rate: u64, config_dir: &std::path::PathBuf, warning_list_msg: &mut Vec<String>) -> std::io::Result<()> {
    let (app, err_track) = App::new(&flags, config_dir);
    log::debug!("Current flags: {:?}", flags);

    if let Some(mut app) = app {
        use ui::setup::*;
        // setup terminal
        setup()?;
        // create app and run it
        run_app(&mut app, tick_rate, err_track, flags, warning_list_msg)?;
        // restore terminal
        restore()?;

        app.save_to_data_file();
    } else {
        err_track.into_iter().for_each(|v| eprintln!("{v}"));
    }
    Ok(())
}

use utils::CfgError;
fn run_app(
    app: &mut App,
    tick_rate: u64,
    err_track: Vec<CfgError>,
    flags: &mut Flags<Flag>,
    warning_list_msg: &mut Vec<String>,
) -> std::io::Result<()> {
    if flags.contains(utils::Flag::FirstInit) {
        warning_list_msg.push("Welcome to ClashTui!".to_string());
        warning_list_msg.push(format!("Please go to config the clashtui_cfg_dir '{}' so that program can work properly", app.clashtui_util.clashtui_dir.to_str().unwrap_or("Failed to get the path")));
    };
    if flags.contains(utils::Flag::ErrorDuringInit) {
        warning_list_msg.push("Some error happened during app init, check the log for detail".to_string());
    }
    err_track
        .into_iter()
        .for_each(|e| app.popup_txt_msg(e.reason));
    log::info!("App init finished");

    use ratatui::{backend::CrosstermBackend, Terminal};
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
    terminal.clear()?;  // Clear terminal residual text before draw.
    let tick_rate = Duration::from_millis(tick_rate);
    use ui::event;
    app.popup_list_msg(warning_list_msg.to_owned());   // Set msg popup before draw
    while !app.should_quit {
        terminal.draw(|f| app.draw(f))?;

        app.late_event();

        if event::poll(tick_rate)? {
            if let Err(e) = app.event(&event::read()?) {
                app.popup_txt_msg(e.to_string())
            };
        }
    }
    log::info!("App Exit");
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
            let clashtui_config_dir_str = env::var("XDG_CONFIG_HOME").map(|p| format!("{}/clashtui", p))
                .or_else(|_| env::var("HOME").map(|home| format!("{}/.config/clashtui", home)))
                .unwrap();
            PathBuf::from(&clashtui_config_dir_str)
        }
    };

    if !clashtui_config_dir.join("config.yaml").exists() {
        use tui::symbols;
        flags.insert(Flag::FirstInit);
        if let Err(err) = crate::utils::init_config(
            &clashtui_config_dir,
            symbols::DEFAULT_BASIC_CLASH_CFG_CONTENT,
        ) {
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
        .encoder(Box::new(PatternEncoder::new("{d(%H:%M:%S)} [{l}] {t} - {m}{n}")))  // Having a timestamp would be better.
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
