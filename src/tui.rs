#[macro_use]
mod key;
mod app;
pub(crate) mod commands;
mod input;
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
    fn on_enter(&mut self) {}
    fn on_leave(&mut self) {}
    fn command_done(&mut self, _event: &commands::CommandEvent) {}
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
