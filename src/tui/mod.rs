/// This module contains the user interface components for the Clash TUI application.
mod frontend;
mod misc;
mod theme;
mod widget;

pub use frontend::{tabs, FrontEnd};
pub use theme::Theme;
pub use widget::PopMsg;

use misc::EventState;

/// A trait for objects that can be drawn on the screen.
trait Drawable {
    /// Renders the object on the given frame within the specified area.
    ///
    /// # Arguments
    ///
    /// * `f` - The frame to render on.
    /// * `area` - The area within the frame to render the object.
    /// * `is_focused` - Indicates whether the object is currently focused.
    fn render(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect, is_focused: bool);

    /// Handles a key event for the object.
    ///
    /// # Arguments
    ///
    /// * `ev` - The key event to handle.
    ///
    /// # Returns
    ///
    /// The state of the event after handling.
    fn handle_key_event(&mut self, ev: &crossterm::event::KeyEvent) -> EventState;
}

#[derive(derive_more::Debug)]
/// Represents different types of calls that can be made to the backend.
pub enum Call {
    Profile(tabs::profile::BackendOp),
    Service(tabs::service::BackendOp),
    #[cfg(feature = "connection-tab")]
    Connection(tabs::connection::BackendOp),
    #[debug("Logs")]
    /// Reads a range of lines from a file.
    ///
    /// From `total_len-start-length` to `total_len-start`
    Logs(usize, usize),
    /// Requests a refresh.
    Tick,
    /// Requests a shutdown.
    Stop,
}

/// Provides functions for setting up and restoring the terminal.
pub mod setup {
    use crossterm::{
        cursor,
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };

    /// Enables raw mode and sets up the terminal for the application.
    pub fn setup() -> Result<(), std::io::Error> {
        enable_raw_mode()?;
        execute!(std::io::stdout(), EnterAlternateScreen, EnableMouseCapture)
    }

    /// Disables raw mode and restores the terminal to its original state.
    pub fn restore() -> Result<(), std::io::Error> {
        disable_raw_mode()?;
        execute!(
            std::io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            cursor::Show
        )
    }

    /// make terminal restorable after panic
    pub fn set_panic_hook() {
        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic| {
            let _ = restore();
            original_hook(panic);
        }));
    }
}
