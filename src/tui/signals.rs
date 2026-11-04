use anyhow::Result;

pub struct Signals {
    _commands: crate::command::SignalGuard,
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
        libc::SIGTSTP => SUSPEND.store(true, Ordering::SeqCst),
        libc::SIGCONT => CONTINUE.store(true, Ordering::SeqCst),
        _ => (),
    }
}

impl Signals {
    pub fn start(cancel: std::sync::Arc<std::sync::atomic::AtomicBool>) -> Result<Self> {
        let commands = crate::command::signal_scope(cancel.clone())?;
        #[cfg(unix)]
        {
            use libc::{SIGCONT, SIGTSTP};
            let mut previous = Vec::new();
            for signal in [SIGTSTP, SIGCONT] {
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
                runtime.spawn(async move {
                    use std::sync::atomic::Ordering;
                    let mut tick = tokio::time::interval(std::time::Duration::from_millis(20));
                    loop {
                        tick.tick().await;
                        if SUSPEND.swap(false, Ordering::SeqCst) && !super::term::is_external() {
                            let cancel = cancel.clone();
                            tokio::task::spawn_blocking(move || {
                                if let Err(error) = super::term::suspend(&cancel) {
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
            Ok(Self {
                _commands: commands,
                previous,
                task,
            })
        }
        #[cfg(not(unix))]
        Ok(Self {
            _commands: commands,
        })
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
    }
}
