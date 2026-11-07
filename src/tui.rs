#[macro_use]
mod key;
mod app;
mod business;
mod core;
mod job;
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
pub(crate) use term::shutdown_requested as command_cancelled;
pub use term::terminal_lease;
pub(crate) use theme::Theme;

trait TuiWidget {
    fn render(
        &mut self,
        f: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        view: business::BusinessView<'_>,
    );
    fn on_enter(&mut self, _business: &mut business::Business) {}
    fn on_leave(&mut self, _business: &mut business::Business) {}
    fn sync_from(&mut self, _view: business::BusinessView<'_>) {}
    fn advance(&mut self) {}
    fn poll_tasks(&mut self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<()> {
        std::task::Poll::Pending
    }
    fn finish_task(&mut self, _visible: bool) -> bool {
        false
    }
    fn has_tasks(&self) -> bool {
        false
    }
    fn needs_animation(&self) -> bool {
        false
    }
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
