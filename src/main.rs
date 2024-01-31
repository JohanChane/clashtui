use anyhow::Result;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::time::{Duration, Instant};

mod app;
mod ui;
mod utils;

use crate::app::App;
use crate::ui::EventState;

/// Mihomo (Clash.Meta) TUI Client
#[derive(Debug, argh::FromArgs)]
struct CliEnv {
    /// time in ms between two ticks.
    #[argh(option, default = "250")]
    tick_rate: u64,
    /// whether unicode symbols are used to improve the overall look of the app
    /// **current, it does Not work
    #[argh(switch)]
    enhanced_graphics: bool,
    /// only update all profiles
    #[argh(switch, short = 'u')]
    update: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli: CliEnv = argh::from_env();
    let mut flags: std::collections::HashMap<utils::Flags, bool> =
        std::collections::HashMap::with_capacity(3);
    flags.insert(utils::Flags::UpdateOnly, cli.update);
    let tick_rate = Duration::from_millis(cli.tick_rate);
    run(flags, tick_rate, cli.enhanced_graphics)?;

    Ok(())
}
#[allow(unused_variables)]
pub fn run(
    flags: std::collections::HashMap<utils::Flags, bool>,
    tick_rate: Duration,
    enhanced_graphics: bool,
) -> Result<()> {
    let res;
    log::debug!("Current flags: {:?}", flags);
    if let Some(mut app) = App::new(flags) {
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
        res = run_app(&mut terminal, &mut app, tick_rate);

        // restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
    } else {
        res = Ok(());
    }

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    tick_rate: Duration,
) -> Result<()> {
    {
        let mut err_tarck = app.get_err_track();
        if *app.flags.get(&utils::Flags::FirstInit).unwrap() {
            app.popup_txt_msg(
                "Welcome to ClashTui(forked)!\n
        Please go to Config Tab to set configs so that program can work properly"
                    .to_string(),
            )
        };
        if *app.flags.get(&utils::Flags::ErrorDuringInit).unwrap() {
            app.popup_txt_msg(
                "Some Error happened during app init, Check the log for detail".to_string(),
            );
        }
        use utils::ClashTuiConfigLoadError;
        loop {
            if !err_tarck.is_empty() {
                let err: Option<ClashTuiConfigLoadError> = err_tarck.pop();
                let el;
                let showstr = match err {
                    Some(v) => {
                        el = match v {
                            ClashTuiConfigLoadError::LoadAppConfig(x) => x.into_string(),
                            ClashTuiConfigLoadError::LoadProfileConfig(x) => x.into_string(),
                            ClashTuiConfigLoadError::LoadClashConfig(x) => x.into_string(),
                        };
                        el
                    }
                    None => panic!("Should not reached arm!!"),
                };
                app.popup_txt_msg(showstr);
                terminal.draw(|f| app.draw(f))?;
            } else {
                break;
            }
        }
    }
    log::info!("App init finished");

    let mut last_tick = Instant::now();
    let mut last_ev = EventState::NotConsumed;
    use crossterm::event;
    loop {
        terminal.draw(|f| app.draw(f))?;

        last_ev = app.handle_last_ev(&last_ev);

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            last_ev = app.event(&event::read()?)?;
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }
        if app.should_quit {
            app.save_config();
            log::info!("App Exit");
            return Ok(());
        }
    }
}
