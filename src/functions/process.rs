//! Process execution shared by configured script actions and the CLI.

use crate::config::scripts::{CommandOutput, ResolvedCommand};
use anyhow::{Context, Result, bail, ensure};
use std::{
    io::{self, Write},
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
#[path = "process/unix.rs"]
mod platform;
#[cfg(windows)]
#[path = "process/windows.rs"]
mod platform;

const CAPTURE_TIMEOUT: Duration = Duration::from_secs(5);
const OUTPUT_LIMIT: usize = 64 * 1024;
// Allows base64 expansion of a 1 MiB input plus terminal framing.
const TERMINAL_OUTPUT_LIMIT: usize = 2 * 1024 * 1024;
const POLL_INTERVAL: Duration = Duration::from_millis(10);
static CANCEL_FOREGROUND: AtomicBool = AtomicBool::new(false);
static SHUTDOWN: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
pub struct CommandOutcome {
    pub status: ExitStatus,
}

#[derive(Debug)]
pub struct CapturedOutcome {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdout_overflow: bool,
    pub stderr_overflow: bool,
}

impl CapturedOutcome {
    pub fn ensure_success(&self) -> Result<()> {
        ensure!(
            self.status.success(),
            "Command exited: {}\n{}",
            self.status,
            self.diagnostic()
        );
        Ok(())
    }

    pub fn diagnostic(&self) -> String {
        let mut text = format!(
            "{}{}",
            display_output(&self.stdout),
            display_output(&self.stderr)
        );
        if self.stdout_overflow || self.stderr_overflow {
            text.push_str("\n[output truncated]");
        }
        text
    }
}

/// Popups and diagnostics contain text, never executable terminal controls.
pub fn display_output(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .chars()
        .flat_map(|c| {
            if c.is_control() && !matches!(c, '\n' | '\r' | '\t') {
                c.escape_default().to_string().chars().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}

pub fn begin_foreground_session() {
    CANCEL_FOREGROUND.store(false, Ordering::SeqCst);
}
pub fn cancel_foreground() {
    CANCEL_FOREGROUND.store(true, Ordering::SeqCst);
}
pub fn request_shutdown() {
    SHUTDOWN.store(true, Ordering::SeqCst);
    cancel_foreground();
}
pub fn foreground_cancelled() -> bool {
    CANCEL_FOREGROUND.load(Ordering::SeqCst) || SHUTDOWN.load(Ordering::SeqCst)
}
pub fn install_console_handler() -> Result<()> {
    platform::install_console_handler()
}
pub fn remove_console_handler() {
    platform::remove_console_handler();
}
pub fn wait_for_enter() -> Result<()> {
    platform::wait_for_enter()
}

struct ManagedChild {
    child: Child,
    group: platform::ProcessGroup,
}

impl ManagedChild {
    fn spawn(
        command: &ResolvedCommand,
        capture: bool,
        piped_input: bool,
        env: &[(&str, &str)],
    ) -> Result<Self> {
        let mut builder = Command::new(&command.program);
        builder
            .args(&command.args)
            .current_dir(&command.cwd)
            .envs(env.iter().copied());
        builder.stdin(if piped_input {
            Stdio::piped()
        } else if capture {
            Stdio::null()
        } else {
            Stdio::inherit()
        });
        if capture {
            builder.stdout(Stdio::piped()).stderr(Stdio::piped());
        } else {
            builder.stdout(Stdio::inherit()).stderr(Stdio::inherit());
        }
        let prepared_group = platform::configure(&mut builder)?;
        let mut child = builder.spawn().with_context(|| {
            format!(
                "Failed to start {:?} in {}",
                command.program,
                command.cwd.display()
            )
        })?;
        let group = match platform::ProcessGroup::attach(&mut child, !capture, prepared_group) {
            Ok(group) => group,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.context("Failed to manage command process group"));
            }
        };
        Ok(Self { child, group })
    }

    fn terminate(&mut self) {
        self.group.terminate();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Drop for ManagedChild {
    fn drop(&mut self) {
        // Also remove descendants left behind by a command which already exited.
        // Actual services are created by their service manager outside this group.
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
    fn drain(&mut self) -> io::Result<bool> {
        if self.eof {
            return Ok(true);
        }
        let mut buffer = [0; 8192];
        for _ in 0..8 {
            let count = match platform::read_pipe(&mut self.reader, &mut buffer) {
                Ok(0) => {
                    self.eof = true;
                    return Ok(true);
                }
                Ok(count) => count,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(true),
                Err(error) => return Err(error),
            };
            let retain = count.min(self.limit - self.bytes.len());
            self.bytes.extend_from_slice(&buffer[..retain]);
            self.overflow |= retain < count;
        }
        Ok(false)
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

/// Caller owns the terminal lease for an interactive command. Input, when supplied,
/// is passed verbatim via stdin; interactive tools can still open the controlling TTY.
pub fn run(
    command: &ResolvedCommand,
    input: Option<Vec<u8>>,
    env: &[(&str, &str)],
    terminal_session: bool,
) -> Result<CapturedOutcome> {
    run_with_options(
        command,
        input,
        env,
        !command.interactive,
        (!command.interactive).then_some(CAPTURE_TIMEOUT),
        terminal_session,
    )
}

pub fn run_foreground(command: &ResolvedCommand) -> Result<CommandOutcome> {
    let outcome = run_with_options(command, None, &[], false, None, true)?;
    Ok(CommandOutcome {
        status: outcome.status,
    })
}

/// Internal data queries always capture, regardless of manual invocation settings.
pub fn run_capture(command: &ResolvedCommand) -> Result<CapturedOutcome> {
    run_capture_with_timeout(command, CAPTURE_TIMEOUT)
}
fn run_capture_with_timeout(
    command: &ResolvedCommand,
    timeout: Duration,
) -> Result<CapturedOutcome> {
    run_with_options(command, None, &[], true, Some(timeout), false)
}

fn run_with_options(
    command: &ResolvedCommand,
    input: Option<Vec<u8>>,
    env: &[(&str, &str)],
    capture: bool,
    timeout: Option<Duration>,
    terminal_session: bool,
) -> Result<CapturedOutcome> {
    let cancelled =
        || SHUTDOWN.load(Ordering::SeqCst) || (terminal_session && foreground_cancelled());
    ensure!(!cancelled(), "Command cancelled");
    let deadline = timeout.map(|timeout| Instant::now() + timeout);
    let mut child = ManagedChild::spawn(command, capture, input.is_some(), env)?;
    let limit = if command.output == CommandOutput::Terminal {
        TERMINAL_OUTPUT_LIMIT
    } else {
        OUTPUT_LIMIT
    };
    let mut stdout = child
        .child
        .stdout
        .take()
        .map(|pipe| Capture::new(pipe, limit))
        .transpose()?;
    let mut stderr = child
        .child
        .stderr
        .take()
        .map(|pipe| Capture::new(pipe, OUTPUT_LIMIT))
        .transpose()?;
    let cancel_write = Arc::new(AtomicBool::new(false));
    let (sender, receiver) = mpsc::channel();
    let writer = if let Some(input) = input {
        let mut stdin = child.child.stdin.take().context("Missing stdin pipe")?;
        platform::prepare_pipe(&stdin).context("Failed to configure command stdin")?;
        let writer_cancel = Arc::clone(&cancel_write);
        Some(thread::spawn(move || {
            let result = write_input(&mut stdin, &input, &writer_cancel);
            drop(stdin); // EOF is part of the script input protocol.
            let _ = sender.send(result);
        }))
    } else {
        None
    };
    let mut written = writer.is_none();
    let mut input_error = None;
    let mut status = None;
    let mut cleaned = false;
    let result = (|| -> Result<()> {
        loop {
            ensure!(!cancelled(), "Command cancelled");
            if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                bail!(
                    "Command timed out after {} ms",
                    timeout.unwrap().as_millis()
                );
            }
            if status.is_none() {
                status = child
                    .child
                    .try_wait()
                    .context("Failed to wait for command")?;
            }
            if !written {
                match receiver.try_recv() {
                    Ok(result) => {
                        // A failing command may close stdin early; preserve its exit status
                        // and diagnostics rather than replacing them with BrokenPipe.
                        input_error = result.err();
                        written = true;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => bail!("Command input writer stopped"),
                    Err(mpsc::TryRecvError::Empty) => (),
                }
            }
            if let Some(status) = status
                && !cleaned
                && (!status.success() || (written && !command.keep_children_on_success))
            {
                child.group.terminate();
                cleaned = true;
            }
            if status.is_some() && written {
                // The parent has finished producing its result. A surviving owner may
                // retain stderr indefinitely; its EOF is not part of command completion.
                if let Some(out) = &mut stdout {
                    out.finish()?;
                }
                if let Some(err) = &mut stderr {
                    err.finish()?;
                }
                return Ok(());
            }
            if let Some(out) = &mut stdout {
                out.drain()?;
            }
            if let Some(err) = &mut stderr {
                err.drain()?;
            }
            thread::sleep(POLL_INTERVAL);
        }
    })();
    if result.is_err() {
        cancel_write.store(true, Ordering::Release);
        child.terminate();
    }
    if let Some(writer) = writer {
        let _ = writer.join();
    }
    result?;
    let status = status.context("Missing command exit status")?;
    if !capture {
        if platform::was_interrupted(status) {
            cancel_foreground();
        }
        println!("Command exited: {status}");
    }
    let outcome = CapturedOutcome {
        status,
        stdout_overflow: stdout.as_ref().is_some_and(|s| s.overflow),
        stderr_overflow: stderr.as_ref().is_some_and(|s| s.overflow),
        stdout: stdout.map_or_else(Vec::new, |s| s.bytes),
        stderr: stderr.map_or_else(Vec::new, |s| s.bytes),
    };
    if status.success()
        && let Some(error) = input_error
    {
        return Err(error).context("Failed to write command input");
    }
    if capture && command.output == CommandOutput::Terminal {
        ensure!(
            !outcome.stdout_overflow,
            "Terminal output exceeded 2 MiB; nothing was forwarded"
        );
    }
    ensure!(!cancelled(), "Command cancelled");
    if status.success() && command.keep_children_on_success {
        child.group.keep_children()?;
    }
    Ok(outcome)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn shell(script: &str) -> ResolvedCommand {
        ResolvedCommand {
            program: "/bin/sh".into(),
            args: vec!["-c".into(), script.into()],
            cwd: std::env::temp_dir(),
            interactive: false,
            ..Default::default()
        }
    }

    #[test]
    fn early_stdin_close_preserves_failure_status_and_diagnostics() {
        let output = run(
            &shell("exec 0<&-; printf diagnostic >&2; sleep 0.1; exit 7"),
            Some(vec![b'x'; 1024 * 1024]),
            &[],
            false,
        )
        .unwrap();
        assert_eq!(output.status.code(), Some(7));
        assert_eq!(output.stderr, b"diagnostic");
        assert!(
            run(
                &shell("exec 0<&-; sleep 0.1; exit 0"),
                Some(vec![b'x'; 1024 * 1024]),
                &[],
                false
            )
            .unwrap_err()
            .to_string()
            .contains("write command input")
        );
    }

    #[test]
    fn terminal_output_preserves_large_sequences_and_rejects_overflow() {
        let mut command = shell("printf '\\033]52;c;'; head -c 1500000 /dev/zero; printf '\\007'");
        command.output = CommandOutput::Terminal;
        let output = run_capture(&command).unwrap();
        assert_eq!(output.stdout.len(), 1500008);
        assert!(output.stdout.starts_with(b"\x1b]52;c;"));
        assert_eq!(output.stdout.last(), Some(&7));
        command.args[1] = "head -c 2200000 /dev/zero".into();
        assert!(
            run_capture(&command)
                .unwrap_err()
                .to_string()
                .contains("nothing was forwarded")
        );
    }

    #[test]
    fn explicit_children_policy_applies_to_all_commands() {
        for keep in [false, true] {
            for exit_code in [0, 7] {
                let marker =
                    std::env::temp_dir().join(format!("clashtui-owner-{}", fastrand::u64(..)));
                let mut command = shell(
                    "(sleep 0.4; printf alive > \"$1\") & printf parent; printf diagnostic >&2; exit \"$2\"",
                );
                command.keep_children_on_success = keep;
                command.args.extend([
                    "test".into(),
                    marker.as_os_str().to_owned(),
                    exit_code.to_string().into(),
                ]);
                let started = Instant::now();
                let outcome = run_capture(&command).unwrap();
                assert!(started.elapsed() < Duration::from_millis(350));
                assert_eq!(outcome.status.code(), Some(exit_code));
                assert_eq!(outcome.stdout, b"parent");
                assert_eq!(outcome.stderr, b"diagnostic");
                thread::sleep(Duration::from_millis(500));
                let survived = marker.exists();
                let _ = std::fs::remove_file(marker);
                assert_eq!(survived, keep && exit_code == 0);
            }
        }
    }

    #[test]
    fn popup_text_cannot_inject_terminal_controls() {
        assert_eq!(
            display_output(b"hello\x1b]52;c;x\x07\n"),
            "hello\\u{1b}]52;c;x\\u{7}\n"
        );
    }

    #[cfg(feature = "tui")]
    #[test]
    fn clipboard_stdin_preserves_utf8_and_metadata_without_shell_expansion() {
        let root = std::env::temp_dir().join(format!("clashtui-copy-{}", fastrand::u64(..)));
        std::fs::create_dir(&root).unwrap();
        let mut command = shell(
            "cat > input; printf '%s/%s/%s' \"$CLASHTUI_COPY_SOURCE\" \"$CLASHTUI_COPY_FORMAT\" \"$1\" > metadata",
        );
        command.cwd = root.clone();
        command
            .args
            .extend(["copy-test".into(), "$(must-not-run)".into()]);
        let text = "中文 'quoted' $HOME\r\n\n".repeat(8192);
        let result = run(
            &command,
            Some(text.clone().into_bytes()),
            &[
                ("CLASHTUI_COPY_SOURCE", "connections"),
                ("CLASHTUI_COPY_FORMAT", "json"),
            ],
            false,
        );
        let input = std::fs::read(root.join("input"));
        let metadata = std::fs::read_to_string(root.join("metadata"));
        std::fs::remove_dir_all(root).unwrap();
        result.unwrap();
        assert_eq!(input.unwrap(), text.as_bytes());
        assert_eq!(metadata.unwrap(), "connections/json/$(must-not-run)");
    }

    #[cfg(feature = "tui")]
    #[test]
    fn clipboard_timeout_includes_a_blocked_stdin_writer() {
        let started = Instant::now();
        let result = run_with_options(
            &shell("exec sleep 30"),
            Some(vec![b'x'; 1024 * 1024]),
            &[],
            true,
            Some(Duration::from_millis(100)),
            false,
        );
        assert!(result.unwrap_err().to_string().contains("timed out"));
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[cfg(feature = "tui")]
    #[test]
    fn clipboard_writer_can_cancel_even_when_a_reader_outlives_the_command() {
        let (mut writer, reader) = std::os::unix::net::UnixStream::pair().unwrap();
        writer.set_nonblocking(true).unwrap();
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let writer_cancel = std::sync::Arc::clone(&cancel);
        let (sender, receiver) = mpsc::channel();
        let thread = thread::spawn(move || {
            let result = write_input(&mut writer, &vec![b'x'; 1024 * 1024], &writer_cancel);
            let _ = sender.send(result);
        });
        thread::sleep(Duration::from_millis(50));
        cancel.store(true, Ordering::Release);
        let result = receiver.recv_timeout(Duration::from_secs(1));
        // Keep the read end alive through the deadline, then release it even on failure.
        drop(reader);
        thread.join().unwrap();
        assert_eq!(
            result.unwrap().unwrap_err().kind(),
            io::ErrorKind::Interrupted
        );
    }

    #[cfg(feature = "tui")]
    #[test]
    fn clipboard_owner_survives_success_but_not_failure() {
        for exit_code in [0, 7] {
            let marker =
                std::env::temp_dir().join(format!("clashtui-copy-owner-{}", fastrand::u64(..)));
            let mut command =
                shell("cat >/dev/null; (sleep 0.3; printf alive > \"$1\") & exit \"$2\"");
            command.args.extend([
                "copy-test".into(),
                marker.as_os_str().to_owned(),
                exit_code.to_string().into(),
            ]);
            command.keep_children_on_success = true;
            let result = run(&command, Some(b"text".to_vec()), &[], false)
                .and_then(|outcome| outcome.ensure_success());
            thread::sleep(Duration::from_millis(500));
            let survived = marker.exists();
            let _ = std::fs::remove_file(marker);
            assert_eq!(result.is_ok(), exit_code == 0);
            assert_eq!(survived, exit_code == 0);
        }
    }

    #[test]
    fn drains_both_streams_beyond_limit_without_deadlocking() {
        let output = run_capture(&shell(
            "(head -c 100000 /dev/zero) & head -c 100000 /dev/zero >&2; wait",
        ))
        .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout.len(), OUTPUT_LIMIT);
        assert_eq!(output.stderr.len(), OUTPUT_LIMIT);
        assert!(output.stdout_overflow && output.stderr_overflow);
    }

    #[test]
    fn exit_status_is_independent_of_output_text() {
        let output = run_capture(&shell("printf 'Error is only text'; exit 0")).unwrap();
        assert!(output.status.success());
        let output = run_capture(&shell("printf 'success'; exit 7")).unwrap();
        assert_eq!(output.status.code(), Some(7));
    }

    #[test]
    fn default_cleanup_does_not_wait_for_descendant_pipe_eof() {
        let started = Instant::now();
        let result =
            run_capture_with_timeout(&shell("sleep 30 & exit 0"), Duration::from_millis(150));
        assert!(result.unwrap().status.success());
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn timeout_kills_the_direct_command() {
        let started = Instant::now();
        let result = run_capture_with_timeout(&shell("exec sleep 30"), Duration::from_millis(100));
        assert!(result.unwrap_err().to_string().contains("timed out"));
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn missing_program_reports_spawn_error() {
        let mut command = shell("");
        command.program = "/clashtui-test-no-such-program".into();
        assert!(
            run_capture(&command)
                .unwrap_err()
                .to_string()
                .contains("Failed to start")
        );
    }

    #[test]
    fn command_keeps_literal_arguments_and_direct_parent() {
        let value = "空格 'quotes' $(must-not-run)";
        let command = ResolvedCommand {
            program: "/bin/sh".into(),
            args: vec![
                "-c".into(),
                "printf '%s\\n%s' \"$PPID\" \"$1\"".into(),
                "test".into(),
                value.into(),
            ],
            cwd: std::env::temp_dir(),
            interactive: false,
            ..Default::default()
        };
        let output = run_capture(&command).unwrap();
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("{}\n{value}", std::process::id())
        );
    }

    #[test]
    fn timed_out_descendant_cannot_continue_after_runner_returns() {
        let marker = std::env::temp_dir().join(format!(
            "clashtui-descendant-{}-{:016x}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let command = ResolvedCommand {
            program: "/bin/sh".into(),
            args: vec![
                "-c".into(),
                "(sleep 0.4; printf escaped > \"$1\") & wait".into(),
                "test".into(),
                marker.as_os_str().to_owned(),
            ],
            cwd: std::env::temp_dir(),
            interactive: false,
            ..Default::default()
        };
        let result = run_capture_with_timeout(&command, Duration::from_millis(100));
        assert!(result.unwrap_err().to_string().contains("timed out"));
        thread::sleep(Duration::from_millis(450));
        let escaped = marker.exists();
        let _ = std::fs::remove_file(marker);
        assert!(!escaped, "Managed descendant survived timeout cleanup");
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;
    use crate::config::{CoreType, scripts::ScriptConfig};

    fn powershell(script: &str) -> ResolvedCommand {
        ResolvedCommand {
            program: "powershell.exe".into(),
            args: ["-NoProfile", "-NonInteractive", "-Command", script]
                .into_iter()
                .map(Into::into)
                .collect(),
            cwd: std::env::temp_dir(),
            interactive: false,
            ..Default::default()
        }
    }

    #[test]
    fn bundled_status_command_runs_from_canonicalized_unicode_path() {
        let root = std::env::temp_dir().join(format!(
            "clashtui script 中文-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let script_dir = root.join("scripts/coresrvctrl/windows");
        std::fs::create_dir_all(&script_dir).unwrap();
        std::fs::write(
            root.join(crate::config::scripts::FILE_NAME),
            include_bytes!("../../contrib/scripts/coresrvctrl/windows/system.json"),
        )
        .unwrap();
        // Exercise the bundled argv with a harmless script instead of the SCM.
        std::fs::write(
            script_dir.join("service.ps1"),
            "\u{feff}param([string]$Operation, [string]$ServiceName)\n\
             if ($Operation -ne 'status' -or $ServiceName -ne 'path test 中文') { exit 3 }\n\
             [Console]::Out.Write('inactive')\n",
        )
        .unwrap();
        let config_root = root.canonicalize().unwrap();
        let config = ScriptConfig::load(&config_root).unwrap();
        let action = config.find(CoreType::Mihomo, "status_core").unwrap();
        let command = config
            .resolve(&config_root, action, |name| {
                assert_eq!(name, "service_name");
                Ok("path test 中文".into())
            })
            .unwrap();
        let result = run_capture(&command);
        std::fs::remove_dir_all(root).unwrap();

        let output = result.unwrap();
        assert!(
            output.status.success(),
            "{}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"inactive");
    }

    #[test]
    fn suspended_command_joins_job_then_runs_and_preserves_exit_status() {
        let output = run_capture(&powershell("[Console]::Out.Write('inactive'); exit 7")).unwrap();
        assert_eq!(output.status.code(), Some(7));
        assert_eq!(output.stdout, b"inactive");
    }

    #[test]
    fn windows_capture_drains_both_streams_after_limits() {
        let output = run_capture(&powershell(
            "[Console]::Out.Write(('x' * 100000)); [Console]::Error.Write(('e' * 100000))",
        ))
        .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout.len(), OUTPUT_LIMIT);
        assert_eq!(output.stderr.len(), OUTPUT_LIMIT);
        assert!(output.stdout_overflow && output.stderr_overflow);
    }

    #[test]
    fn windows_timeout_terminates_the_job() {
        let start = Instant::now();
        let result = run_capture_with_timeout(
            &powershell("Start-Sleep -Seconds 30"),
            Duration::from_millis(200),
        );
        assert!(result.unwrap_err().to_string().contains("timed out"));
        assert!(start.elapsed() < Duration::from_secs(5));
    }
}
