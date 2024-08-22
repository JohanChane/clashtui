mod frontend;
mod misc;
mod theme;
mod widget;

pub use frontend::{tabs, FrontEnd};
pub use theme::Theme;

use misc::EventState;

trait Drawable {
    fn render(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect, is_fouced: bool);
    fn handle_key_event(&mut self, ev: &crossterm::event::KeyEvent) -> EventState;
}

/// Wrap the caller,
/// the inner ops are defined in their own files.
pub enum Call {
    Service(tabs::service::BackendOp),
    Tick,
    Stop,
}

enum PopMsg {
    Ask(Vec<String>),
    Processing(Vec<String>),
    Notice(Vec<String>),
}

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
