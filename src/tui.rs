mod agent;
mod app;
pub mod binding;
pub(crate) mod global_keymap;
mod key;
mod popmsg;
mod signals;
mod tab;
mod term;
mod theme;
mod utils;
mod widget;

pub use app::App;
pub use key::Key;
pub(crate) use key::format_key_sequence;
pub use term::hold;
pub(crate) use theme::Theme;

trait TuiWidget {
    /// Default no-op -- only PopUp overrides this. Tab key dispatch goes
    /// through ChordHandler -> dispatch_by_seq -> Keymap::find_by_seq.
    fn handle_key_event(&mut self, _kv: &Key) {}
    fn render(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect);
    fn sync(&mut self);
    fn on_enter(&mut self) {}
    fn on_leave(&mut self) {}
}

pub fn init() -> anyhow::Result<()> {
    agent::init()?;
    theme::Theme::load();
    term::setup()
}

pub fn restore() -> anyhow::Result<()> {
    term::teardown();
    Ok(())
}
