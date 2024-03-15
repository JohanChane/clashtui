#![warn(clippy::all)]
use core::time::Duration;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::time::Instant;
mod app;
mod tui;
mod utils;

use crate::app::App;
use crate::tui::EventState;
use crate::utils::{Flag, Flags};

pub const VERSION: &str = concat!(env!("CLASHTUI_VERSION"));

/// Mihomo (Clash.Meta) TUI Client
#[derive(Debug, argh::FromArgs)]
struct CliEnv {
    /// time in ms between two ticks.
    #[argh(option, default = "250")]
    tick_rate: u64,
    /// don't show UI but only update all profiles
    #[argh(switch, short = 'u')]
    update_all_profiles: bool,
    /// print version information and exit
    #[argh(switch, short = 'v')]
    version: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let CliEnv {
        tick_rate,
        update_all_profiles,
        version,
    } = argh::from_env();
    if version {
        println!("{VERSION}");
        std::process::exit(0);
    }
    let mut flags = Flags::empty();
    if update_all_profiles {
        flags.insert(utils::Flag::UpdateOnly);
    };
    let tick_rate = Duration::from_millis(tick_rate);
    run(flags, tick_rate)?;

    Ok(())
}
pub fn run(mut flags: Flags<Flag>, tick_rate: Duration) -> std::io::Result<()> {
    let res;
    let config_dir = load_app_dir(&mut flags);

    setup_logging(config_dir.join("clashtui.log").to_str().unwrap());

    let (app, err_track) = App::new(&flags, &config_dir);
    log::debug!("Current flags: {:?}", flags);
    if let Some(mut app) = app {
        use ui::setup::*;
        // setup terminal
        setup()?;
        let terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

        // create app and run it
        res = run_app(terminal, &mut app, tick_rate, err_track, flags);

        // restore terminal
        restore()?;

        app.save(config_dir.join("config.yaml").to_str().unwrap())?;
    } else {
        if !err_track.is_empty() {
            err_track.into_iter().for_each(|v| eprintln!("{v}"));
        }
        res = Ok(());
    }

    if let Err(err) = res {
        eprintln!("{err:?}");
    }

    Ok(())
}

use utils::CfgError;
fn run_app<B: Backend>(
    mut terminal: Terminal<B>,
    app: &mut App,
    tick_rate: Duration,
    mut err_track: Vec<CfgError>,
    flags: Flags<Flag>,
) -> std::io::Result<()> {
    {
        if flags.contains(utils::Flag::FirstInit) {
            app.popup_txt_msg("Welcome to ClashTui(forked)!".to_string());
            app.popup_txt_msg(
                "Please go to Config Tab to set configs so that program can work properly"
                    .to_string(),
            );
        };
        if flags.contains(utils::Flag::ErrorDuringInit) {
            app.popup_txt_msg(
                "Some Error happened during app init, Check the log for detail".to_string(),
            );
        }
        while !err_track.is_empty() {
            let err: Option<CfgError> = err_track.pop();
            let showstr = match err {
                Some(v) => v.reason.to_string(),
                None => unreachable!(),
            };
            app.popup_txt_msg(showstr);
        }
    }
    log::info!("App init finished");

    let mut last_tick = Instant::now();
    let mut last_ev = EventState::NotConsumed;
    use ui::event;
    while !app.should_quit {
        terminal.draw(|f| app.draw(f))?;

        last_ev = app.handle_last_ev(&last_ev);

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if ui::event::poll(timeout)? {
            last_ev = app
                .event(&event::read()?)
                .map_err(|e| app.popup_txt_msg(e.to_string()))
                .unwrap_or(EventState::NotConsumed);
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
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
