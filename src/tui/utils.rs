pub mod raw_mode {
    //! Provides functions for setting up and restoring the terminal.

    use crossterm::{
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    };

    /// Enables raw mode and sets up the terminal for the application.
    pub fn setup() -> Result<(), std::io::Error> {
        enable_raw_mode()?;
        execute!(std::io::stdout(), EnterAlternateScreen)
    }

    /// Disables raw mode and restores the terminal to its original state.
    pub fn restore() -> Result<(), std::io::Error> {
        disable_raw_mode()?;
        execute!(
            std::io::stdout(),
            LeaveAlternateScreen,
            crossterm::cursor::Show
        )
    }

    /// make terminal restorable after panic
    ///
    /// The hook must never write to stdout/stderr. When the terminal is gone
    /// (e.g. its pty returns `EIO`), a failed `println!`/`eprintln!` inside the
    /// hook would panic again, and with `panic = "abort"` that aborts the
    /// process and loses the original panic location. Instead the payload and
    /// location are appended to `clashtui.log` with the same file the logger
    /// writes to. Any failure while logging the panic is ignored.
    pub fn set_panic_hook() {
        std::panic::set_hook(Box::new(move |panic| {
            let _ = restore();
            log_panic(panic);
        }));
    }

    /// Append the panic payload and location to `config_dir/clashtui.log`.
    ///
    /// Deliberately uses `writeln!` on a `File` (which never panics on write
    /// errors) instead of a print macro.
    fn log_panic(panic: &std::panic::PanicHookInfo<'_>) {
        use std::io::Write;

        let Some(dir) = crate::config::config_dir_opt() else {
            return;
        };
        let path = dir.join("clashtui.log");
        let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        else {
            return;
        };

        let payload = panic.payload();
        let msg = if let Some(s) = payload.downcast_ref::<&str>() {
            (*s).to_owned()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "non-string panic payload".to_owned()
        };
        // Keep the panic on a single line so it is easy to grep.
        let msg = msg.replace(['\n', '\r'], " ");
        let location = panic
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "<unknown>".to_owned());

        let _ = writeln!(file, "PANIC {location}: {msg}");
    }
}
