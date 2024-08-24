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
    fn handle_key_event(&mut self, ev: &crossterm::event::KeyEvent) -> EventState;
}

/// Wrap the caller,
/// the inner ops are defined in their own files.
pub enum Call {
    Profile(tabs::profile::BackendOp),
    Service(tabs::service::BackendOp),
    /// read file by lines, from `total_len-start-length` to `total_len-start`
    Logs(usize, usize),
    /// ask backend for clash infos
    Infos,
    /// ask to refresh
    Tick,
    /// ask to shutdown
    Stop,
}

impl std::fmt::Display for Call {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Call::Profile(_) => "Profile",
                Call::Service(_) => "Service",
                Call::Logs(..) => "Logs",
                Call::Infos => "Infos",
                Call::Tick => "Tick",
                Call::Stop => "Stop",
            }
        )
    }
}

/// turn terminal from/into Raw mode
pub mod setup {
    use crossterm::{
        cursor,
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    pub fn setup() -> Result<(), std::io::Error> {
        enable_raw_mode()?;
        execute!(std::io::stdout(), EnterAlternateScreen, EnableMouseCapture)
    }
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
