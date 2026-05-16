use anyhow::Result;

pub struct Signals;

impl Signals {
    pub fn start() -> Result<Self> {
        #[cfg(unix)]
        {
            use libc::{SIGCONT, SIGHUP, SIGINT, SIGQUIT, SIGTERM, SIGTSTP};

            let mut sys = signal_hook_tokio::Signals::new([
                SIGINT, SIGQUIT, SIGHUP, SIGTERM, SIGTSTP, SIGCONT,
            ])?;

            tokio::spawn(async move {
                use futures_lite::StreamExt as _;
                while let Some(n) = sys.next().await {
                    if n == SIGINT {
                        // ignored — Ctrl-C is a keyboard event in raw mode
                    } else if n == SIGQUIT || n == SIGHUP || n == SIGTERM {
                        super::app::QUIT.store(true, std::sync::atomic::Ordering::Relaxed);
                        break;
                    } else if n == SIGTSTP {
                        let _ = crate::tui::hold(true);
                        unsafe {
                            libc::kill(0, SIGTSTP);
                        }
                        _ = crate::tui::hold(false);
                        super::app::FULL_RENDER.notify_one();
                    }
                }
            });
        }
        Ok(Self)
    }
}
