use utils::*;

mod agent;
mod app;
mod key;
mod popmsg;
mod signals;
mod tab;
mod theme;
mod utils;
mod widget;

pub use app::App;
pub use key::Key;
pub use theme::Theme;

trait TuiWidget {
    fn handle_key_event(&mut self, kv: &Key);
    fn render(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect);
    fn sync(&mut self);
}

pub fn init() -> anyhow::Result<()> {
    agent::init()?;
    theme::Theme::load();
    raw_mode::setup()?;
    raw_mode::set_panic_hook();
    Ok(())
}

pub fn restore() -> anyhow::Result<()> {
    raw_mode::restore()?;
    Ok(())
}

/// Leave RawMode and get back to main screen
pub fn hold(on: bool) -> anyhow::Result<()> {
    if on {
        raw_mode::restore()?;
        // tell ratatui to re-render
        app::FULL_RENDER.notify_one();
    } else {
        raw_mode::setup()?
    }
    Ok(())
}
