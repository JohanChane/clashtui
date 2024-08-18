#![allow(clippy::new_without_default)]
pub use crossterm::event;
/// Visibility-related functions, can be impl using `derive`
///
/// Require `is_visible:bool` in the struct
pub trait Visibility {
    fn is_visible(&self) -> bool;
    fn show(&mut self);
    fn hide(&mut self);
    fn set_visible(&mut self, b: bool);
}
pub mod utils;
pub mod widgets;
pub use utils::{EventState, Infailable, Theme};

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
