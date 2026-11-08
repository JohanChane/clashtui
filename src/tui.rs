#[macro_use]
mod key;
mod app;
pub(crate) mod commands;
mod event;
mod input;
mod monitor;
mod ops;
mod page;
mod popmsg;
mod signals;
mod tab;
mod term;
mod theme;
mod utils;
mod widget;

pub use app::App;
use key::Key;
pub use key::init as keymap_init;
pub(crate) use theme::Theme;

trait TuiWidget {
    fn render(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect);
    fn sync(&mut self);
}

pub fn init() -> anyhow::Result<()> {
    key::load()?;
    theme::Theme::load();
    if let Err(error) = crate::scripts::init(&crate::config::config_dir_path()) {
        log::warn!("{error:#}");
    }
    term::setup()
}

pub fn restore() -> anyhow::Result<()> {
    term::teardown();
    Ok(())
}
