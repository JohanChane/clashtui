mod frontend;
mod misc;
mod theme;
mod widget;

pub use frontend::{tabs, FrontEnd};
pub use theme::Theme;
pub use widget::PopMsg;

use misc::EventState;

trait Drawable {
    fn render(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect, is_fouced: bool);
    /// - unrecognized event -> [EventState::NotConsumed]
    fn handle_key_event(&mut self, ev: &crossterm::event::KeyEvent) -> EventState;
}
#[derive(derive_more::Debug)]
/// Wrap the caller,
/// the inner ops are defined in their own files.
pub enum Call {
    #[debug("Profile")]
    Profile(tabs::profile::BackendOp),
    #[debug("Service")]
    Service(tabs::service::BackendOp),
    #[cfg(feature = "connection-tab")]
    #[debug("Connection")]
    Connection(tabs::connection::BackendOp),
    #[debug("Logs")]
    /// read file by lines, from `total_len-start-length` to `total_len-start`
    Logs(usize, usize),
    /// ask backend for clash infos
    Infos,
    /// ask to refresh
    Tick,
    /// ask to shutdown
    Stop,
}

/// turn terminal from/into Raw mode
pub mod setup {
    use crossterm::{
        cursor,
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    /// Enable raw mode.
    pub fn setup() -> Result<(), std::io::Error> {
        enable_raw_mode()?;
        execute!(std::io::stdout(), EnterAlternateScreen, EnableMouseCapture)
    }
    /// Disable raw mode.
    pub fn restore() -> Result<(), std::io::Error> {
        disable_raw_mode()?;
        execute!(
            std::io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            cursor::Show
        )
    }
}
