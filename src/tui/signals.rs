use anyhow::Result;

pub struct Signals {
    #[cfg(unix)]
    task: Option<tokio::task::JoinHandle<()>>,
    #[cfg(unix)]
    stop: Box<dyn Fn() + Send + Sync>,
}

impl Signals {
    pub fn start() -> Result<Self> {
        #[cfg(unix)]
        {
            use libc::{SIGCONT, SIGHUP, SIGINT, SIGQUIT, SIGTERM, SIGTSTP};
            let mut signals = signal_hook_tokio::Signals::new([
                SIGINT, SIGQUIT, SIGHUP, SIGTERM, SIGTSTP, SIGCONT,
            ])?;
            let handle = signals.handle();
            let task = tokio::spawn(async move {
                use futures_lite::StreamExt as _;
                let mut handoffs = tokio::task::JoinSet::new();
                loop {
                    tokio::select! {
                        signal = signals.next() => match signal {
                            Some(SIGINT) => {
                                // ignored — Ctrl-C is a keyboard event in raw mode
                                // While lent, the foreground child receives the signal itself.
                            }
                            Some(SIGQUIT | SIGHUP | SIGTERM) => {
                                super::app::QUIT.store(true, std::sync::atomic::Ordering::Relaxed);
                                break;
                            }
                            Some(SIGTSTP) if handoffs.is_empty() => {
                                handoffs.spawn(super::term::run_blocking(|| -> Result<()> {
                                    let lease = super::term::terminal_lease()?;
                                    if !lease.is_cancelled() {
                                        // SIGTSTP has a handler installed; SIGSTOP performs
                                        // the actual job-control stop without reentering it.
                                        unsafe { libc::raise(libc::SIGSTOP); }
                                    }
                                    lease.release()
                                }));
                            }
                            Some(_) => {}
                            None => break,
                        },
                        done = handoffs.join_next(), if !handoffs.is_empty() => {
                            if let Some(Ok(Err(error))) = done {
                                log::warn!("Terminal suspension: {error:#}");
                            }
                        }
                    }
                }
                while handoffs.join_next().await.is_some() {}
            });
            Ok(Self {
                task: Some(task),
                stop: Box::new(move || handle.close()),
            })
        }
        #[cfg(not(unix))]
        Ok(Self {})
    }

    pub fn close(&self) {
        #[cfg(unix)]
        (self.stop)();
    }

    pub async fn shutdown(self) {
        #[cfg(unix)]
        {
            let mut signals = self;
            (signals.stop)();
            if let Some(task) = signals.task.take() {
                let _ = task.await;
            }
        }
    }
}

impl Drop for Signals {
    fn drop(&mut self) {
        #[cfg(unix)]
        (self.stop)();
    }
}
