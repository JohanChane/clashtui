use std::sync::atomic::AtomicBool;

use utils::*;

mod agent;
mod app;
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
pub use term::hold;
pub use theme::Theme;

pub async fn prompt_sudo_password() -> Option<String> {
    match popmsg::input::InputMasked::new()
        .with_title("Sudo Password".to_owned())
        .with_prompt("Sudo password:".to_owned())
        .build_and_send()
        .await
    {
        Ok(pw) => Some(pw),
        Err(_) => None,
    }
}

pub static EXT_PROC: AtomicBool = AtomicBool::new(false);

trait TuiWidget {
    fn handle_key_event(&mut self, kv: &Key);
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

pub fn suspend_terminal() {
    term::suspend();
}

pub fn resume_terminal() -> anyhow::Result<()> {
    term::resume()
}
