use super::commands::TaskId;
use super::{
    input::InputReader,
    utils::{TerminalState, raw_mode},
};
use crate::command::{CommandError, CommandResult};
use anyhow::{Context, Result, bail};
use std::time::Duration;
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

pub(crate) fn is_external() -> bool {
    EXTERNAL.load(Ordering::SeqCst)
}

pub(super) struct Request {
    task: TaskId,
    ready: mpsc::SyncSender<CommandResult<TerminalState>>,
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

    pub fn close(&mut self) {
        *COORDINATOR.lock().unwrap_or_else(|e| e.into_inner()) = None;
        self.requests.close();
        while let Ok(request) = self.requests.try_recv() {
            let _ = request.ready.send(Err(CommandError::cancelled()));
        }
    }

    pub async fn handoff(
        &mut self,
        request: Request,
        input: &mut InputReader,
        cancel: &AtomicBool,
    ) -> Result<()> {
        log::trace!("Terminal handoff for task {}", request.task);
        let paused = tokio::select! {
            result = input.pause() => result,
            _ = wait_cancel(cancel) => Err(CommandError::cancelled().into()),
        };
        if let Err(error) = paused {
            let _ = input.resume();
            let _ = request.ready.send(Err(CommandError::io(error)));
            return Ok(());
        }
        let state = match raw_mode::restore().and_then(|()| TerminalState::capture()) {
            Ok(state) => state,
            Err(error) => {
                let _ = raw_mode::setup();
                let _ = input.resume();
                let _ = request.ready.send(Err(CommandError::io(error)));
                return Ok(());
            }
        };
        let mut restore = RestoreTerminal(Some(state.clone()));
        EXTERNAL.store(true, Ordering::SeqCst);
        if !cancel.load(Ordering::Acquire) && request.ready.send(Ok(state)).is_ok() {
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
        self.close();
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
    tui: TuiLease,
}

struct TuiLease {
    release: Option<oneshot::Sender<()>>,
    normal_modes: TerminalState,
}

impl TerminalLease {
    fn acquire(task: TaskId, cancel: &AtomicBool) -> CommandResult<Self> {
        if cancel.load(Ordering::Acquire) {
            return Err(CommandError::cancelled());
        }
        let lock = match TERMINAL_LOCK.try_lock() {
            Ok(lock) => lock,
            Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => {
                return Err(CommandError::io(anyhow::anyhow!(
                    "Another terminal operation is running"
                )));
            }
        };
        let coordinator = COORDINATOR
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let coordinator = coordinator.ok_or_else(CommandError::cancelled)?;
        let (ready, receiver) = mpsc::sync_channel(1);
        let (release, released) = oneshot::channel();
        coordinator
            .send(Request {
                task,
                ready,
                released,
            })
            .map_err(|_| CommandError::cancelled())?;
        loop {
            if cancel.load(Ordering::Acquire) {
                return Err(CommandError::cancelled());
            }
            match receiver.recv_timeout(Duration::from_millis(10)) {
                Ok(result) => {
                    let normal_modes = result?;
                    if cancel.load(Ordering::Acquire) {
                        return Err(CommandError::cancelled());
                    }
                    return Ok(Self {
                        _lock: lock,
                        tui: TuiLease {
                            release: Some(release),
                            normal_modes,
                        },
                    });
                }
                Err(mpsc::RecvTimeoutError::Timeout) => (),
                Err(mpsc::RecvTimeoutError::Disconnected) => return Err(CommandError::cancelled()),
            }
        }
    }
}

impl Drop for TerminalLease {
    fn drop(&mut self) {
        if let Some(release) = self.tui.release.take() {
            let _ = release.send(());
        }
    }
}

async fn wait_cancel(cancel: &AtomicBool) {
    while !cancel.load(Ordering::Acquire) {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

pub fn with_terminal<T>(
    task: TaskId,
    cancel: &AtomicBool,
    job: impl FnOnce() -> CommandResult<T>,
) -> CommandResult<T> {
    let lease = TerminalLease::acquire(task, cancel)?;
    if cancel.load(Ordering::Acquire) {
        return Err(CommandError::cancelled());
    }
    let result = job();
    if let Err(error) = lease.tui.normal_modes.restore() {
        return Err(match result {
            Err(original) => original.with_cleanup(error),
            Ok(_) => CommandError::io(error),
        });
    }
    if let Err(error) = &result {
        eprintln!("{error}");
    }
    if !cancel.load(Ordering::Acquire) && !crate::command::shutdown_requested() {
        println!("Press Enter to return to clashtui...");
        let _ = std::io::stdout().flush();
        if let Err(error) = wait_for_enter(cancel) {
            return Err(match result {
                Err(original) => original.with_cleanup(error),
                Ok(_) => CommandError::io(error),
            });
        }
    }
    drop(lease);
    result
}

#[cfg(unix)]
pub(super) fn suspend(cancel: &AtomicBool) -> Result<()> {
    let _lease = TerminalLease::acquire(0, cancel)?;
    if unsafe { libc::raise(libc::SIGSTOP) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

#[cfg(unix)]
fn wait_for_enter(cancel: &AtomicBool) -> Result<()> {
    use std::io::{self, IsTerminal};
    if !io::stdin().is_terminal() {
        return Ok(());
    }
    // Ctrl-C can flush tty input after poll reports readiness. The following
    // read must stay nonblocking so cancellation cannot get stuck in that gap.
    struct RestoreFlags(libc::c_int);
    impl Drop for RestoreFlags {
        fn drop(&mut self) {
            unsafe { libc::fcntl(libc::STDIN_FILENO, libc::F_SETFL, self.0) };
        }
    }
    let flags = unsafe { libc::fcntl(libc::STDIN_FILENO, libc::F_GETFL) };
    if flags < 0
        || unsafe { libc::fcntl(libc::STDIN_FILENO, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
    {
        return Err(io::Error::last_os_error().into());
    }
    let _restore = RestoreFlags(flags);
    while !cancel.load(Ordering::Acquire) {
        let mut fd = libc::pollfd {
            fd: libc::STDIN_FILENO,
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut fd, 1, 50) };
        if ready < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error.into());
        }
        if ready > 0 {
            let mut buffer = [0u8; 256];
            let count =
                unsafe { libc::read(libc::STDIN_FILENO, buffer.as_mut_ptr().cast(), buffer.len()) };
            if count == 0 {
                return Ok(());
            }
            if count < 0 {
                let error = io::Error::last_os_error();
                if matches!(
                    error.kind(),
                    io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
                ) {
                    continue;
                }
                return Err(error.into());
            }
            if buffer[..count as usize]
                .iter()
                .any(|byte| matches!(byte, b'\n' | b'\r'))
            {
                return Ok(());
            }
        }
    }
    Ok(())
}

#[cfg(windows)]
fn wait_for_enter(cancel: &AtomicBool) -> Result<()> {
    use std::io::{self, IsTerminal};
    use windows_sys::Win32::{
        Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::{
            Console::{GetStdHandle, INPUT_RECORD, KEY_EVENT, ReadConsoleInputW, STD_INPUT_HANDLE},
            Threading::WaitForSingleObject,
        },
    };
    if !io::stdin().is_terminal() {
        return Ok(());
    }
    let input = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    while !cancel.load(Ordering::Acquire) {
        match unsafe { WaitForSingleObject(input, 50) } {
            WAIT_OBJECT_0 => {
                let mut record = INPUT_RECORD::default();
                let mut read = 0;
                if unsafe { ReadConsoleInputW(input, &mut record, 1, &mut read) } == 0 {
                    return Err(io::Error::last_os_error().into());
                }
                if read == 1
                    && u32::from(record.EventType) == KEY_EVENT
                    && unsafe { record.Event.KeyEvent.bKeyDown } != 0
                    && unsafe { record.Event.KeyEvent.wVirtualKeyCode } == 13
                {
                    return Ok(());
                }
            }
            WAIT_TIMEOUT => (),
            _ => return Err(io::Error::last_os_error().into()),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    static SERIAL: Mutex<()> = Mutex::new(());

    #[test]
    fn waiting_handoff_cancels_without_a_main_loop_acknowledgement() {
        let _serial = SERIAL.lock().unwrap();
        let mut coordinator = Coordinator::register().unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let task_cancel = cancel.clone();
        let worker = std::thread::spawn(move || {
            with_terminal(42, &task_cancel, || -> CommandResult<()> {
                panic!("cancelled job started")
            })
        });
        let request = coordinator.requests.blocking_recv().unwrap();
        assert_eq!(request.task, 42);
        cancel.store(true, Ordering::Release);
        assert_eq!(
            worker.join().unwrap().unwrap_err().kind,
            crate::command::CommandErrorKind::Cancelled
        );
        // A late coordinator cannot authorize a worker which already cancelled.
        assert!(request.ready.send(Err(CommandError::cancelled())).is_err());
    }
    #[test]
    fn coordinator_close_unblocks_queued_handoff_as_cancelled() {
        let _serial = SERIAL.lock().unwrap();
        let mut coordinator = Coordinator::register().unwrap();
        let worker = std::thread::spawn(|| {
            with_terminal(43, &AtomicBool::new(false), || -> CommandResult<()> {
                panic!("closed coordinator started job")
            })
        });
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while coordinator.requests.is_empty() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        coordinator.close();
        assert_eq!(
            worker.join().unwrap().unwrap_err().kind,
            crate::command::CommandErrorKind::Cancelled
        );
    }
}
