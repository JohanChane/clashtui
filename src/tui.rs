#[macro_use]
mod key;
mod app;
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
pub use term::hold;
pub(crate) use theme::Theme;

trait TuiWidget {
    fn handle_key_event(&mut self, kv: &Key);
    fn render(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect);
    fn sync(&mut self);
    fn on_enter(&mut self) {}
    fn on_leave(&mut self) {}
}

pub fn init() -> anyhow::Result<()> {
    key::load()?;
    theme::Theme::load();
    term::setup()
}

pub fn restore() -> anyhow::Result<()> {
    term::teardown();
    Ok(())
}
