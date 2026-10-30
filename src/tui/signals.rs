use anyhow::Result;

pub struct Signals {
    #[cfg(unix)]
    previous: Vec<(libc::c_int, libc::sigaction)>,
    #[cfg(unix)]
    task: Option<tokio::task::JoinHandle<()>>,
}

#[cfg(unix)]
static SUSPEND: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
#[cfg(unix)]
static CONTINUE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(unix)]
extern "C" fn receive(signal: libc::c_int) {
    use std::sync::atomic::Ordering;
    // Only lock-free atomic stores/loads are allowed in the signal handler.
    // Terminal changes, task creation and logging happen in normal execution.
    match signal {
        libc::SIGINT => {
            if super::term::is_external() {
                crate::functions::process::cancel_foreground();
            }
        }
        libc::SIGQUIT | libc::SIGHUP | libc::SIGTERM => {
            crate::functions::process::request_shutdown();
            crate::tui::request_quit();
        }
        libc::SIGTSTP => SUSPEND.store(true, Ordering::SeqCst),
        libc::SIGCONT => CONTINUE.store(true, Ordering::SeqCst),
        _ => (),
    }
}

impl Signals {
    pub fn start() -> Result<Self> {
        crate::functions::process::install_console_handler()?;
        #[cfg(unix)]
        {
            use libc::{SIGCONT, SIGHUP, SIGINT, SIGQUIT, SIGTERM, SIGTSTP};
            let mut previous = Vec::new();
            for signal in [SIGINT, SIGQUIT, SIGHUP, SIGTERM, SIGTSTP, SIGCONT] {
                let mut action = unsafe { std::mem::zeroed::<libc::sigaction>() };
                let mut old = unsafe { std::mem::zeroed::<libc::sigaction>() };
                action.sa_sigaction = receive as *const () as usize;
                action.sa_flags = libc::SA_RESTART;
                unsafe {
                    libc::sigemptyset(&mut action.sa_mask);
                }
                if unsafe { libc::sigaction(signal, &action, &mut old) } != 0 {
                    let error = std::io::Error::last_os_error();
                    for (registered, old) in &previous {
                        unsafe {
                            libc::sigaction(*registered, old, std::ptr::null_mut());
                        }
                    }
                    return Err(error.into());
                }
                previous.push((signal, old));
            }
            let task = tokio::runtime::Handle::try_current().ok().map(|runtime| {
                runtime.spawn(async {
                    use std::sync::atomic::Ordering;
                    let mut tick = tokio::time::interval(std::time::Duration::from_millis(20));
                    loop {
                        tick.tick().await;
                        if SUSPEND.swap(false, Ordering::SeqCst) && !super::term::is_external() {
                            tokio::task::spawn_blocking(|| {
                                if let Err(error) = super::term::suspend() {
                                    log::warn!("Could not suspend terminal: {error:#}");
                                }
                            });
                        }
                        if CONTINUE.swap(false, Ordering::SeqCst) {
                            super::app::FULL_RENDER.notify_one();
                        }
                    }
                })
            });
            Ok(Self { previous, task })
        }
        #[cfg(not(unix))]
        Ok(Self {})
    }
}

impl Drop for Signals {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            if let Some(task) = &self.task {
                task.abort();
            }
            for (signal, action) in &self.previous {
                unsafe {
                    libc::sigaction(*signal, action, std::ptr::null_mut());
                }
            }
        }
        crate::functions::process::remove_console_handler();
    }
}
