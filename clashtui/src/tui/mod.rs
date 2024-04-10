mod statusbar;
mod symbols;
mod tabbar;
mod tabs;
mod utils;
extern crate ui;
use ui::utils::tools;
use ui::{widgets, EventState, Theme, Visibility};

use statusbar::StatusBar;
use tabbar::TabBar;

mod app;
pub use app::App;


const TICK_RATE: u64 = 250;
use crate::utils::{Flag,Flags};
pub fn run_app(
    app: &mut App,
    err_track: Vec<crate::utils::CfgError>,
    flags: Flags<Flag>,
) -> std::io::Result<()> {
    use core::time::Duration;
    if flags.contains(Flag::FirstInit) {
        app.popup_txt_msg("Welcome to ClashTui(forked)!".to_string());
        app.popup_txt_msg(
            "Please go to Config Tab to set configs so that program can work properly".to_string(),
        );
    };
    if flags.contains(Flag::ErrorDuringInit) {
        app.popup_txt_msg(
            "Some Error happened during app init, Check the log for detail".to_string(),
        );
    }
    err_track
        .into_iter()
        .for_each(|e| app.popup_txt_msg(e.reason));
    log::info!("App init finished");

    use ratatui::{backend::CrosstermBackend, Terminal};
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
    let tick_rate = Duration::from_millis(TICK_RATE);
    use ui::event;
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