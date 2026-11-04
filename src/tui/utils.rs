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

/// Preserve normal terminal modes across external programs, including programs
/// interrupted after they changed echo or console input processing.
#[derive(Clone)]
pub(super) struct TerminalState {
    #[cfg(unix)]
    state: libc::termios,
    #[cfg(windows)]
    modes: Vec<(isize, u32)>,
}

impl TerminalState {
    pub fn capture() -> std::io::Result<Self> {
        #[cfg(unix)]
        {
            let mut state = unsafe { std::mem::zeroed::<libc::termios>() };
            if unsafe { libc::tcgetattr(libc::STDIN_FILENO, &mut state) } == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(Self { state })
        }
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Console::{
                GetConsoleMode, GetStdHandle, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
            };
            let mut modes = Vec::new();
            for kind in [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE] {
                let handle = unsafe { GetStdHandle(kind) };
                let mut mode = 0;
                if unsafe { GetConsoleMode(handle, &mut mode) } == 0 {
                    return Err(std::io::Error::last_os_error());
                }
                modes.push((handle as isize, mode));
            }
            Ok(Self { modes })
        }
    }

    pub fn restore(&self) -> std::io::Result<()> {
        #[cfg(unix)]
        if unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &self.state) } == -1 {
            return Err(std::io::Error::last_os_error());
        }
        #[cfg(windows)]
        for (handle, mode) in &self.modes {
            if unsafe {
                windows_sys::Win32::System::Console::SetConsoleMode(
                    *handle as windows_sys::Win32::Foundation::HANDLE,
                    *mode,
                )
            } == 0
            {
                return Err(std::io::Error::last_os_error());
            }
        }
        Ok(())
    }
}
