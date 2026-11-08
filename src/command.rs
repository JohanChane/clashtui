//! Synchronous process execution. No script or UI policy belongs here.

use anyhow::{Context, Result};
use std::{
    ffi::{OsStr, OsString},
    io::{self, Write},
    path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

#[cfg(unix)]
#[path = "command/unix.rs"]
mod platform;
#[cfg(windows)]
#[path = "command/windows.rs"]
mod platform;

const POLL_INTERVAL: Duration = Duration::from_millis(10);
const OUTPUT_LIMIT: usize = 64 * 1024;
static INTERRUPTED: AtomicBool = AtomicBool::new(false);
static SHUTDOWN: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
pub struct CommandSpec {
    pub program: OsString,
    pub args: Vec<OsString>,
    pub cwd: Option<PathBuf>,
    pub stdin: Input,
    pub output: ProcessOutputMode,
    pub timeout: Option<Duration>,
    pub keep_children_on_success: bool,
}

#[derive(Debug)]
pub enum Input {
    Inherit,
    Closed,
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone, Copy)]
pub enum ProcessOutputMode {
    Inherit,
    Capture {
        stdout_limit: usize,
        stderr_limit: usize,
        fail_on_stdout_overflow: bool,
    },
}

#[derive(Debug, Default)]
pub struct CapturedOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
}

#[derive(Debug)]
pub struct ProcessOutput {
    pub status: ExitStatus,
    pub captured: CapturedOutput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandErrorKind {
    Cancelled,
    TimedOut,
    Spawn,
    Io,
    OutputLimit,
}

#[derive(Debug)]
pub struct CommandError {
    pub kind: CommandErrorKind,
    pub status: Option<ExitStatus>,
    pub captured: CapturedOutput,
    pub cause: Option<anyhow::Error>,
}

impl CommandError {
    pub fn cancelled() -> Self {
        Self::new(CommandErrorKind::Cancelled, None)
    }
    pub fn io(error: impl Into<anyhow::Error>) -> Self {
        match error.into().downcast::<Self>() {
            Ok(error) => error,
            Err(error) => Self::new(CommandErrorKind::Io, Some(error)),
        }
    }
    pub fn with_cleanup(mut self, error: impl std::fmt::Display) -> Self {
        let message = format!("Cleanup failed: {error}");
        self.cause = Some(match self.cause.take() {
            Some(cause) => cause.context(message),
            None => anyhow::anyhow!(message),
        });
        self
    }

    fn new(kind: CommandErrorKind, cause: Option<anyhow::Error>) -> Self {
        Self {
            kind,
            status: None,
            captured: CapturedOutput::default(),
            cause,
        }
    }
}
impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self.kind {
            CommandErrorKind::Cancelled => "Command cancelled",
            CommandErrorKind::TimedOut => "Command timed out",
            CommandErrorKind::Spawn => "Failed to start command",
            CommandErrorKind::Io => "Command I/O failed",
            CommandErrorKind::OutputLimit => "Command output limit exceeded; nothing was forwarded",
        };
        write!(f, "{message}")?;
        if let Some(cause) = &self.cause {
            write!(f, ": {cause:#}")?;
        }
        Ok(())
    }
}
impl std::error::Error for CommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.cause.as_ref().map(|cause| cause.as_ref())
    }
}
pub type CommandResult<T> = std::result::Result<T, CommandError>;

pub fn capture(
    program: &OsStr,
    args: &[OsString],
    cancel: &AtomicBool,
) -> CommandResult<ProcessOutput> {
    run(
        CommandSpec {
            program: program.into(),
            args: args.into(),
            cwd: None,
            stdin: Input::Closed,
            output: ProcessOutputMode::Capture {
                stdout_limit: OUTPUT_LIMIT,
                stderr_limit: OUTPUT_LIMIT,
                fail_on_stdout_overflow: false,
            },
            timeout: None,
            keep_children_on_success: false,
        },
        cancel,
    )
}

#[cfg(unix)]
pub fn inherit(
    program: &OsStr,
    args: &[OsString],
    cancel: &AtomicBool,
) -> CommandResult<ExitStatus> {
    run(
        CommandSpec {
            program: program.into(),
            args: args.into(),
            cwd: None,
            stdin: Input::Inherit,
            output: ProcessOutputMode::Inherit,
            timeout: None,
            keep_children_on_success: false,
        },
        cancel,
    )
    .map(|output| output.status)
}

struct ManagedChild {
    child: Child,
    group: platform::ProcessGroup,
}
impl ManagedChild {
    fn spawn(spec: &CommandSpec) -> Result<Self> {
        let mut builder = Command::new(&spec.program);
        builder.args(&spec.args);
        if let Some(cwd) = &spec.cwd {
            builder.current_dir(cwd);
        }
        builder.stdin(match spec.stdin {
            Input::Inherit => Stdio::inherit(),
            Input::Closed => Stdio::null(),
            Input::Bytes(_) => Stdio::piped(),
        });
        let inherited = matches!(spec.output, ProcessOutputMode::Inherit);
        if inherited {
            builder.stdout(Stdio::inherit()).stderr(Stdio::inherit());
        } else {
            builder.stdout(Stdio::piped()).stderr(Stdio::piped());
        }
        let prepared = platform::configure(&mut builder)?;
        let mut child = builder
            .spawn()
            .with_context(|| format!("Failed to start {:?}", spec.program))?;
        match platform::ProcessGroup::attach(&mut child, inherited, prepared) {
            Ok(group) => Ok(Self { child, group }),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                Err(error)
            }
        }
    }
    fn terminate(&mut self) -> Option<ExitStatus> {
        self.group.terminate();
        let _ = self.child.kill();
        self.child.wait().ok()
    }
}
impl Drop for ManagedChild {
    fn drop(&mut self) {
        self.terminate();
    }
}
struct Capture<R> {
    reader: R,
    bytes: Vec<u8>,
    limit: usize,
    overflow: bool,
    eof: bool,
}
impl<R: platform::Pipe> Capture<R> {
    fn new(reader: R, limit: usize) -> io::Result<Self> {
        platform::prepare_pipe(&reader)?;
        Ok(Self {
            reader,
            bytes: Vec::new(),
            limit,
            overflow: false,
            eof: false,
        })
    }

    /// Drain available bytes without waiting for EOF from surviving descendants.
    /// Bound each pass so an endlessly writing command cannot starve its deadline.
    fn drain(&mut self) -> io::Result<()> {
        if self.eof {
            return Ok(());
        }
        let mut buffer = [0; 8192];
        for _ in 0..8 {
            let count = match platform::read_pipe(&mut self.reader, &mut buffer) {
                Ok(0) => {
                    self.eof = true;
                    return Ok(());
                }
                Ok(count) => count,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                Err(error) => return Err(error),
            };
            let retain = count.min(self.limit - self.bytes.len());
            self.bytes.extend_from_slice(&buffer[..retain]);
            self.overflow |= retain < count;
        }
        Ok(())
    }

    fn finish(&mut self) -> io::Result<()> {
        // Snapshot the tail once. A daemon that keeps writing must not extend
        // command completion indefinitely or force us to wait for its EOF.
        let mut pending = platform::pending_bytes(&self.reader)?;
        let mut buffer = [0; 8192];
        while pending > 0 {
            let size = pending.min(buffer.len());
            let count = match platform::read_pipe(&mut self.reader, &mut buffer[..size]) {
                Ok(0) => break,
                Ok(count) => count,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => return Err(error),
            };
            let retain = count.min(self.limit - self.bytes.len());
            self.bytes.extend_from_slice(&buffer[..retain]);
            self.overflow |= retain < count;
            pending -= count;
        }
        Ok(())
    }
}

fn write_input(
    writer: &mut impl Write,
    mut remaining: &[u8],
    cancelled: &AtomicBool,
) -> io::Result<()> {
    while !remaining.is_empty() {
        if cancelled.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Command input cancelled",
            ));
        }
        match writer.write(remaining) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(written) => remaining = &remaining[written..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => thread::sleep(POLL_INTERVAL),
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

pub fn run(spec: CommandSpec, cancel: &AtomicBool) -> CommandResult<ProcessOutput> {
    if cancel.load(Ordering::Acquire) {
        return Err(CommandError::cancelled());
    }
    let deadline = spec
        .timeout
        .map(|timeout| {
            Instant::now()
                .checked_add(timeout)
                .context("Command timeout is too large")
        })
        .transpose()
        .map_err(CommandError::io)?;
    let mut child = ManagedChild::spawn(&spec)
        .map_err(|error| CommandError::new(CommandErrorKind::Spawn, Some(error)))?;
    let (stdout_limit, stderr_limit, fail_overflow) = match spec.output {
        ProcessOutputMode::Inherit => (0, 0, false),
        ProcessOutputMode::Capture {
            stdout_limit,
            stderr_limit,
            fail_on_stdout_overflow,
        } => (stdout_limit, stderr_limit, fail_on_stdout_overflow),
    };
    let mut stdout = child
        .child
        .stdout
        .take()
        .map(|pipe| Capture::new(pipe, stdout_limit))
        .transpose()
        .map_err(CommandError::io)?;
    let mut stderr = child
        .child
        .stderr
        .take()
        .map(|pipe| Capture::new(pipe, stderr_limit))
        .transpose()
        .map_err(CommandError::io)?;
    let cancel_write = Arc::new(AtomicBool::new(false));
    let (sender, receiver) = mpsc::channel();
    let writer = if let Input::Bytes(input) = spec.stdin {
        let mut stdin = child.child.stdin.take().expect("piped stdin");
        platform::prepare_pipe(&stdin).map_err(CommandError::io)?;
        let writer_cancel = cancel_write.clone();
        Some(thread::spawn(move || {
            let result = write_input(&mut stdin, &input, &writer_cancel);
            drop(stdin);
            let _ = sender.send(result);
        }))
    } else {
        None
    };
    let mut written = writer.is_none();
    let mut input_error = None;
    let mut status = None;
    let mut cleaned = false;
    let mut result = (|| -> CommandResult<()> {
        loop {
            if cancel.load(Ordering::Acquire) {
                return Err(CommandError::cancelled());
            }
            if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                return Err(CommandError::new(CommandErrorKind::TimedOut, None));
            }
            if status.is_none() {
                status = child.child.try_wait().map_err(CommandError::io)?;
            }
            if !written {
                match receiver.try_recv() {
                    Ok(result) => {
                        input_error = result.err();
                        written = true;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        return Err(CommandError::io(anyhow::anyhow!(
                            "Command input writer stopped"
                        )));
                    }
                    Err(mpsc::TryRecvError::Empty) => (),
                }
            }
            // A writer failure is final even if a child keeps running. Prefer an
            // already available nonzero exit, never wait indefinitely for one.
            if written && input_error.is_some() && status.is_none() {
                status = child.child.try_wait().map_err(CommandError::io)?;
                if status.is_none() {
                    return Err(CommandError::io(input_error.take().unwrap()));
                }
            }
            if let Some(status) = status
                && !cleaned
                && (!status.success() || (written && !spec.keep_children_on_success))
            {
                child.group.terminate();
                cleaned = true;
            }
            if status.is_some() && written {
                if let Some(out) = &mut stdout {
                    out.finish().map_err(CommandError::io)?;
                }
                if let Some(err) = &mut stderr {
                    err.finish().map_err(CommandError::io)?;
                }
            } else {
                if let Some(out) = &mut stdout {
                    out.drain().map_err(CommandError::io)?;
                }
                if let Some(err) = &mut stderr {
                    err.drain().map_err(CommandError::io)?;
                }
            }
            if fail_overflow && stdout.as_ref().is_some_and(|out| out.overflow) {
                return Err(CommandError::new(CommandErrorKind::OutputLimit, None));
            }
            if let Some(status) = status
                && written
            {
                if matches!(spec.output, ProcessOutputMode::Inherit)
                    && platform::was_interrupted(status)
                {
                    cancel.store(true, Ordering::Release);
                    return Err(CommandError::cancelled());
                }
                if status.success()
                    && let Some(error) = input_error.take()
                {
                    return Err(CommandError::io(error));
                }
                if cancel.load(Ordering::Acquire) {
                    return Err(CommandError::cancelled());
                }
                return Ok(());
            }
            thread::sleep(POLL_INTERVAL);
        }
    })();
    if result.is_ok()
        && status.is_some_and(|status| status.success())
        && spec.keep_children_on_success
    {
        result = child.group.keep_children().map_err(CommandError::io);
    }
    if result.is_err() {
        cancel_write.store(true, Ordering::Release);
        status = status.or_else(|| child.terminate());
        // Always clean the managed group even if try_wait already reaped the parent.
        child.terminate();
        if let Some(out) = &mut stdout
            && let Err(error) = out.finish()
        {
            result = result.map_err(|original| original.with_cleanup(error));
        }
        if let Some(err) = &mut stderr
            && let Err(error) = err.finish()
        {
            result = result.map_err(|original| original.with_cleanup(error));
        }
    }
    if let Some(writer) = writer {
        #[cfg(windows)]
        if result.is_err() {
            platform::cancel_writer(&writer);
        }
        let _ = writer.join();
    }
    let captured = CapturedOutput {
        stdout_truncated: stdout.as_ref().is_some_and(|out| out.overflow),
        stderr_truncated: stderr.as_ref().is_some_and(|out| out.overflow),
        stdout: stdout.map_or_else(Vec::new, |out| out.bytes),
        stderr: stderr.map_or_else(Vec::new, |out| out.bytes),
    };
    match result {
        Ok(()) => Ok(ProcessOutput {
            status: status.expect("finished command"),
            captured,
        }),
        Err(mut error) => {
            error.status = status;
            error.captured = captured;
            Err(error)
        }
    }
}

/// Signals only publish atomic flags. A short-lived monitor forwards them to the
/// caller's cancellation token without accessing shared pointers in a handler.
pub struct SignalGuard {
    registration: Option<platform::Signals>,
    stop: Arc<AtomicBool>,
    monitor: Option<thread::JoinHandle<()>>,
}
pub fn shutdown_requested() -> bool {
    SHUTDOWN.load(Ordering::Acquire)
}
pub fn signal_scope(cancel: Arc<AtomicBool>) -> Result<SignalGuard> {
    INTERRUPTED.store(false, Ordering::Release);
    if shutdown_requested() {
        cancel.store(true, Ordering::Release);
    }
    let registration = platform::Signals::install()?;
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let monitor = thread::spawn(move || {
        loop {
            if INTERRUPTED.swap(false, Ordering::AcqRel) || shutdown_requested() {
                cancel.store(true, Ordering::Release);
            }
            if stopped.load(Ordering::Acquire) {
                break;
            }
            thread::sleep(POLL_INTERVAL);
        }
    });
    Ok(SignalGuard {
        registration: Some(registration),
        stop,
        monitor: Some(monitor),
    })
}
impl Drop for SignalGuard {
    fn drop(&mut self) {
        self.registration.take();
        self.stop.store(true, Ordering::Release);
        if let Some(monitor) = self.monitor.take() {
            let _ = monitor.join();
        }
    }
}

/// Popups and diagnostics contain text, never executable terminal controls.
pub fn display_output(bytes: &[u8]) -> String {
    let mut text = String::new();
    for c in String::from_utf8_lossy(bytes).chars() {
        if c.is_control() && !matches!(c, '\n' | '\r' | '\t') {
            text.extend(c.escape_default());
        } else {
            text.push(c);
        }
    }
    text
}

impl CapturedOutput {
    pub fn diagnostic(&self) -> String {
        let mut text = format!(
            "{}{}",
            display_output(&self.stdout),
            display_output(&self.stderr)
        );
        if self.stdout_truncated || self.stderr_truncated {
            text.push_str("\n[output truncated]");
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(script: &str) -> CommandSpec {
        #[cfg(unix)]
        let (program, args) = ("/bin/sh", vec!["-c".into(), script.into()]);
        #[cfg(windows)]
        let (program, args) = (
            "powershell.exe",
            vec![
                "-NoProfile".into(),
                "-NonInteractive".into(),
                "-Command".into(),
                script.into(),
            ],
        );
        CommandSpec {
            program: program.into(),
            args,
            cwd: Some(std::env::temp_dir()),
            stdin: Input::Closed,
            output: ProcessOutputMode::Capture {
                stdout_limit: OUTPUT_LIMIT,
                stderr_limit: OUTPUT_LIMIT,
                fail_on_stdout_overflow: false,
            },
            timeout: Some(Duration::from_secs(5)),
            keep_children_on_success: false,
        }
    }
    fn execute(spec: CommandSpec) -> CommandResult<ProcessOutput> {
        run(spec, &AtomicBool::new(false))
    }

    #[test]
    fn spawn_failure_is_distinct_from_cancellation() {
        let mut spec = command("");
        spec.program = "/clashtui-test-no-such-program".into();
        let error = execute(spec).unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::Spawn);
        assert!(error.status.is_none());
        let error = run(command(""), &AtomicBool::new(true)).unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::Cancelled);
        assert!(error.status.is_none());
    }
    #[test]
    fn captured_exit_130_is_not_a_cancellation() {
        let output = execute(command("exit 130")).unwrap();
        assert_eq!(output.status.code(), Some(130));
    }
    #[test]
    fn diagnostics_escape_terminal_controls() {
        assert_eq!(
            display_output(b"hello\x1b]52;c;x\x07\n"),
            "hello\\u{1b}]52;c;x\\u{7}\n"
        );
    }
    #[test]
    fn timeout_retains_partial_output_and_cancels_blocked_input() {
        #[cfg(unix)]
        let script = "printf before; printf problem >&2; exec sleep 30";
        #[cfg(windows)]
        let script = "[Console]::Out.Write('before'); [Console]::Error.Write('problem'); Start-Sleep -Seconds 30";
        let mut spec = command(script);
        spec.stdin = Input::Bytes(vec![b'x'; 1024 * 1024]);
        // PowerShell startup needs substantially more time on Windows CI.
        spec.timeout = Some(Duration::from_secs(2));
        spec.keep_children_on_success = true;
        let start = Instant::now();
        let error = execute(spec).unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::TimedOut);
        assert_eq!(error.captured.stdout, b"before");
        assert_eq!(error.captured.stderr, b"problem");
        assert!(start.elapsed() < Duration::from_secs(5));
    }
    #[test]
    fn drains_both_streams_and_marks_truncation() {
        #[cfg(unix)]
        let script = "(head -c 100000 /dev/zero) & head -c 100000 /dev/zero >&2; wait; exit 7";
        #[cfg(windows)]
        let script =
            "[Console]::Out.Write(('x' * 100000)); [Console]::Error.Write(('e' * 100000)); exit 7";
        let output = execute(command(script)).unwrap();
        assert_eq!(output.status.code(), Some(7));
        assert_eq!(output.captured.stdout.len(), OUTPUT_LIMIT);
        assert_eq!(output.captured.stderr.len(), OUTPUT_LIMIT);
        assert!(output.captured.stdout_truncated && output.captured.stderr_truncated);
    }
    #[cfg(unix)]
    #[test]
    fn stdin_is_utf8_literal_data_and_completion_includes_eof() {
        let text = "中文 'quotes' $(must-not-run) ${path}\r\n".repeat(4096);
        let mut spec = command("cat; printf '%s' \"$1\" >&2");
        spec.stdin = Input::Bytes(text.clone().into_bytes());
        spec.args.extend(["test".into(), "$(literal)".into()]);
        spec.output = ProcessOutputMode::Capture {
            stdout_limit: text.len(),
            stderr_limit: 100,
            fail_on_stdout_overflow: true,
        };
        let output = execute(spec).unwrap();
        assert_eq!(output.captured.stdout, text.as_bytes());
        assert_eq!(output.captured.stderr, b"$(literal)");
    }
    #[cfg(unix)]
    #[test]
    fn early_stdin_close_preserves_nonzero_exit() {
        for exit in [0, 7] {
            let mut spec = command(&format!("exec 0<&-; printf diagnostic >&2; exit {exit}"));
            spec.stdin = Input::Bytes(vec![b'x'; 1024 * 1024]);
            let output = execute(spec);
            if exit == 0 {
                let error = output.unwrap_err();
                assert_eq!(error.kind, CommandErrorKind::Io);
                assert_eq!(error.captured.stderr, b"diagnostic");
            } else {
                assert_eq!(output.unwrap().status.code(), Some(7));
            }
        }
    }
    #[cfg(unix)]
    #[test]
    fn cancellation_retains_diagnostics_and_reaps_managed_children() {
        let marker = std::env::temp_dir().join(format!("clashtui-cancel-{}", fastrand::u64(..)));
        let mut spec = command("printf ready; (sleep 0.6; printf escaped > \"$1\") & wait");
        spec.args.extend(["test".into(), marker.as_os_str().into()]);
        let cancel = AtomicBool::new(false);
        let error = thread::scope(|scope| {
            scope.spawn(|| {
                thread::sleep(Duration::from_millis(150));
                cancel.store(true, Ordering::Release);
            });
            run(spec, &cancel).unwrap_err()
        });
        assert_eq!(error.kind, CommandErrorKind::Cancelled);
        assert_eq!(error.captured.stdout, b"ready");
        thread::sleep(Duration::from_millis(650));
        assert!(!marker.exists());
    }
    #[cfg(unix)]
    #[test]
    fn retained_children_do_not_extend_output_collection() {
        for keep in [false, true] {
            for exit in [0, 7] {
                let marker =
                    std::env::temp_dir().join(format!("clashtui-owner-{}", fastrand::u64(..)));
                let mut spec = command(
                    "cat >/dev/null; (sleep 0.5; printf alive > \"$1\") & printf parent; exit \"$2\"",
                );
                spec.stdin = Input::Bytes(b"input".to_vec());
                spec.args.extend([
                    "test".into(),
                    marker.as_os_str().into(),
                    exit.to_string().into(),
                ]);
                spec.keep_children_on_success = keep;
                let start = Instant::now();
                let output = execute(spec).unwrap();
                assert!(start.elapsed() < Duration::from_millis(450));
                assert_eq!(output.captured.stdout, b"parent");
                thread::sleep(Duration::from_millis(600));
                let survived = marker.exists();
                let _ = std::fs::remove_file(marker);
                assert_eq!(survived, keep && exit == 0);
            }
        }
    }
    #[cfg(unix)]
    #[test]
    fn large_output_is_captured_without_truncation() {
        let mut spec = command("head -c 1500000 /dev/zero");
        spec.output = ProcessOutputMode::Capture {
            stdout_limit: 2 * 1024 * 1024,
            stderr_limit: OUTPUT_LIMIT,
            fail_on_stdout_overflow: true,
        };
        let output = execute(spec).unwrap();
        assert!(output.status.success());
        assert_eq!(output.captured.stdout.len(), 1500000);
        assert!(output.captured.stdout.iter().all(|&byte| byte == 0));
        assert!(!output.captured.stdout_truncated);
    }
    #[cfg(unix)]
    #[test]
    fn output_overflow_cleans_children() {
        use std::{io::Read, os::unix::fs::OpenOptionsExt};

        struct CleanupFifo(PathBuf);
        impl Drop for CleanupFifo {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let fifo = std::env::temp_dir().join(format!("clashtui-overflow-{}", fastrand::u64(..)));
        nix::unistd::mkfifo(
            &fifo,
            nix::sys::stat::Mode::S_IRUSR | nix::sys::stat::Mode::S_IWUSR,
        )
        .unwrap();
        let fifo = CleanupFifo(fifo);
        let mut reader = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&fifo.0)
            .unwrap();
        // Open before forking so the background child holds the FIFO throughout
        // output collection. EOF then proves cleanup without racing a timer
        // against the time needed to transfer more than 2 MiB on a busy runner.
        let mut spec =
            command("exec 3>\"$1\"; printf ready >&3; sleep 30 & head -c 2200000 /dev/zero; wait");
        spec.args.extend(["test".into(), fifo.0.as_os_str().into()]);
        spec.keep_children_on_success = true;
        spec.output = ProcessOutputMode::Capture {
            stdout_limit: 2 * 1024 * 1024,
            stderr_limit: OUTPUT_LIMIT,
            fail_on_stdout_overflow: true,
        };
        let error = execute(spec).unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::OutputLimit);
        assert!(error.captured.stdout_truncated);
        let mut ready = [0; 5];
        reader.read_exact(&mut ready).unwrap();
        assert_eq!(&ready, b"ready");
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match reader.read(&mut [0]) {
                Ok(0) => break,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    assert!(
                        Instant::now() < deadline,
                        "child still holds the FIFO after cleanup"
                    );
                    thread::sleep(POLL_INTERVAL);
                }
                result => panic!("unexpected FIFO read after cleanup: {result:?}"),
            }
        }
    }
    #[test]
    fn writer_cancels_while_an_unmanaged_reader_keeps_the_pipe_open() {
        let (reader, mut writer) = std::io::pipe().unwrap();
        #[cfg(unix)]
        platform::prepare_pipe(&writer).unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let writer_cancel = cancel.clone();
        let worker = thread::spawn(move || {
            write_input(&mut writer, &vec![b'x'; 1024 * 1024], &writer_cancel).unwrap_err();
        });
        thread::sleep(Duration::from_millis(100));
        cancel.store(true, Ordering::Release);
        #[cfg(windows)]
        platform::cancel_writer(&worker);
        let deadline = Instant::now() + Duration::from_secs(2);
        while !worker.is_finished() && Instant::now() < deadline {
            thread::sleep(POLL_INTERVAL);
        }
        let finished = worker.is_finished();
        drop(reader);
        worker.join().unwrap();
        assert!(finished, "writer waited for unmanaged reader to close");
    }
}
