pub mod raw_mode {
    //! Provides functions for setting up and restoring the terminal.

    #[cfg(unix)]
    use crossterm::event::{
        KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    };
    use crossterm::{
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    };

    /// Enables raw mode and sets up the terminal for the application.
    pub fn setup() -> Result<(), std::io::Error> {
        enable_raw_mode()?;
        execute!(std::io::stdout(), EnterAlternateScreen)?;
        // Crossterm's Windows backend rejects both keyboard enhancement commands.
        #[cfg(unix)]
        execute!(
            std::io::stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        )?;
        Ok(())
    }

    /// Disables raw mode and restores the terminal to its original state.
    pub fn restore() -> Result<(), std::io::Error> {
        disable_raw_mode()?;
        #[cfg(unix)]
        execute!(std::io::stdout(), PopKeyboardEnhancementFlags)?;
        execute!(
            std::io::stdout(),
            LeaveAlternateScreen,
            crossterm::cursor::Show
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
