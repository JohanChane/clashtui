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
    let cli: CliEnv = argh::from_env();
    if cli.version {
        println!("{VERSION}");
        std::process::exit(0);
    }
    let mut flags = Flags::empty();
    if cli.update_all_profiles {
        flags.insert(utils::Flag::UpdateOnly);
    };
    let tick_rate = Duration::from_millis(cli.tick_rate);
    run(flags, tick_rate)?;

    Ok(())
}
pub fn run(mut flags: Flags<Flag>, tick_rate: Duration) -> std::io::Result<()> {
    let res;
    let config_dir = load_app_dir(&mut flags);
    let (app, err_track) = App::new(&flags, &config_dir);
    log::debug!("Current flags: {:?}", flags);
    if let Some(mut app) = app {
        use crossterm::{
            event::{DisableMouseCapture, EnableMouseCapture},
            execute,
            terminal::{
                disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
            },
        };
        // setup terminal
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // create app and run it
        res = run_app(&mut terminal, &mut app, tick_rate, err_track, flags);

        // restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
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
    terminal: &mut Terminal<B>,
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
    use crossterm::event;
    while !app.should_quit {
        terminal.draw(|f| app.draw(f))?;

        last_ev = app.handle_last_ev(&last_ev);

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
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
