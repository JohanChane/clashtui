use super::{
    input::InputReader,
    utils::{TerminalState, raw_mode},
};
use anyhow::{Context, Result, bail};
use std::{
    io::Write,
    sync::{
        Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};
use tokio::sync::{mpsc as async_mpsc, oneshot};

static TERMINAL_LOCK: Mutex<()> = Mutex::new(());
static COORDINATOR: Mutex<Option<async_mpsc::UnboundedSender<Request>>> = Mutex::new(None);
static EXTERNAL: AtomicBool = AtomicBool::new(false);

pub fn setup() -> Result<()> {
    raw_mode::setup()?;
    raw_mode::set_panic_hook();
    Ok(())
}

pub fn teardown() {
    let _ = raw_mode::restore();
}

pub(crate) fn is_running() -> bool {
    COORDINATOR
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .is_some()
}

pub(crate) fn is_external() -> bool {
    EXTERNAL.load(Ordering::SeqCst)
}

pub(super) struct Request {
    ready: mpsc::SyncSender<Result<TerminalState>>,
    released: oneshot::Receiver<()>,
}

pub(super) struct Coordinator {
    requests: async_mpsc::UnboundedReceiver<Request>,
}

impl Coordinator {
    pub fn register() -> Result<Self> {
        let (sender, requests) = async_mpsc::unbounded_channel();
        let mut current = COORDINATOR.lock().unwrap_or_else(|e| e.into_inner());
        if current.is_some() {
            bail!("Terminal coordinator already registered");
        }
        *current = Some(sender);
        Ok(Self { requests })
    }

    pub async fn next(&mut self) -> Option<Request> {
        self.requests.recv().await
    }

    pub async fn handoff(&mut self, request: Request, input: &mut InputReader) -> Result<()> {
        if let Err(error) = input.pause().await {
            let _ = request.ready.send(Err(error));
            return Ok(());
        }
        let state = match raw_mode::restore().and_then(|()| TerminalState::capture()) {
            Ok(state) => state,
            Err(error) => {
                let _ = raw_mode::setup();
                let _ = input.resume();
                let _ = request.ready.send(Err(error.into()));
                return Ok(());
            }
        };
        let mut restore = RestoreTerminal(Some(state.clone()));
        EXTERNAL.store(true, Ordering::SeqCst);
        if request.ready.send(Ok(state)).is_ok() {
            // Draw and input polling stay stopped until the worker releases ownership.
            let _ = request.released.await;
        }
        EXTERNAL.store(false, Ordering::SeqCst);
        let modes = restore.0.take().unwrap().restore();
        let setup = raw_mode::setup();
        let resume = input.resume();
        modes.context("Failed to restore terminal modes")?;
        setup.context("Failed to restore TUI terminal")?;
        resume?;
        Ok(())
    }
}

impl Drop for Coordinator {
    fn drop(&mut self) {
        *COORDINATOR.lock().unwrap_or_else(|e| e.into_inner()) = None;
        EXTERNAL.store(false, Ordering::SeqCst);
    }
}

struct RestoreTerminal(Option<TerminalState>);
impl Drop for RestoreTerminal {
    fn drop(&mut self) {
        if let Some(state) = self.0.take() {
            EXTERNAL.store(false, Ordering::SeqCst);
            let _ = state.restore();
            let _ = raw_mode::setup();
        }
    }
}

struct TerminalLease {
    _lock: MutexGuard<'static, ()>,
    tui: Option<TuiLease>,
}

struct TuiLease {
    release: oneshot::Sender<()>,
    normal_modes: TerminalState,
}

impl TerminalLease {
    fn acquire() -> Result<Self> {
        let lock = match TERMINAL_LOCK.try_lock() {
            Ok(lock) => lock,
            Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => {
                bail!("Another terminal operation is running")
            }
        };
        let coordinator = COORDINATOR
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let tui = if let Some(coordinator) = coordinator {
            let (ready, receiver) = mpsc::sync_channel(1);
            let (release, released) = oneshot::channel();
            coordinator
                .send(Request { ready, released })
                .map_err(|_| anyhow::anyhow!("Terminal coordinator stopped"))?;
            let modes = receiver
                .recv()
                .context("Terminal coordinator stopped before handoff")??;
            Some(TuiLease {
                release,
                normal_modes: modes,
            })
        } else {
            None
        };
        Ok(Self { _lock: lock, tui })
    }

    fn is_tui(&self) -> bool {
        self.tui.is_some()
    }
}

impl Drop for TerminalLease {
    fn drop(&mut self) {
        if let Some(tui) = self.tui.take() {
            let _ = tui.release.send(());
        } else {
            EXTERNAL.store(false, Ordering::SeqCst);
        }
    }
}

/// Run on a blocking worker, once per operation (including composed actions).
/// Before a TUI coordinator is registered this is a normal CLI operation.
pub fn with_terminal<T>(job: impl FnOnce() -> Result<T>) -> Result<T> {
    let lease = TerminalLease::acquire()?;
    crate::functions::process::begin_foreground_session();
    let _signals = if !lease.is_tui() {
        EXTERNAL.store(true, Ordering::SeqCst);
        Some(super::signals::Signals::start()?)
    } else {
        None
    };
    let result = job();
    if let Some(tui) = &lease.tui {
        // A command may leave the terminal raw or disable CR-to-LF translation.
        // Restore normal modes before reading the acknowledgement line.
        if let Err(error) = tui.normal_modes.restore() {
            eprintln!("Failed to restore normal terminal modes: {error}");
            drop(lease);
            return result.and(Err(error.into()));
        }
        if let Err(error) = &result {
            eprintln!("{error:#}");
        }
        if !crate::functions::process::foreground_cancelled() {
            println!("Press Enter to return to clashtui...");
            let _ = std::io::stdout().flush();
            if let Err(error) = crate::functions::process::wait_for_enter() {
                eprintln!("Failed to wait for Enter: {error:#}");
            }
        }
    }
    drop(lease);
    result
}

#[cfg(unix)]
pub(super) fn suspend() -> Result<()> {
    let _lease = TerminalLease::acquire()?;
    // SIGSTOP cannot recurse through the SIGTSTP handler. The shell resumes us
    // with SIGCONT; releasing the lease then restores the TUI.
    if unsafe { libc::raise(libc::SIGSTOP) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_error_releases_lease_and_does_not_require_enter() {
        let error = with_terminal(|| -> Result<()> { bail!("test failure") }).unwrap_err();
        assert_eq!(error.to_string(), "test failure");
        assert_eq!(with_terminal(|| Ok(42)).unwrap(), 42);
    }
}
