#[macro_use]
mod key;
mod app;
mod clipboard;
mod input;
mod popmsg;
pub(crate) mod script_output;
mod signals;
mod tab;
mod term;
mod theme;
mod utils;
mod widget;

pub use app::App;
use key::Key;
pub use key::init as keymap_init;
#[cfg(windows)]
pub(crate) use term::is_external;
pub(crate) use term::is_running;
pub use term::with_terminal;
pub(crate) use theme::Theme;

trait TuiWidget {
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

pub(crate) fn request_quit() {
    app::QUIT.store(true, std::sync::atomic::Ordering::SeqCst);
}
