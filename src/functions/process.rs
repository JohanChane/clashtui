//! Process execution shared by configured service actions and the CLI.

use crate::config::scripts::ResolvedCommand;
use anyhow::{Context, Result, bail};
use std::{
    io::{self, Read},
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
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
    fn spawn(command: &ResolvedCommand, capture: bool) -> Result<Self> {
        let mut builder = Command::new(&command.program);
        builder.args(&command.args).current_dir(&command.cwd);
        if capture {
            builder
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
        } else {
            builder
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
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

/// Caller owns the terminal lease. This also works directly in a CLI session.
pub fn run_foreground(command: &ResolvedCommand) -> Result<CommandOutcome> {
    if foreground_cancelled() {
        bail!("Command cancelled");
    }
    let mut child = ManagedChild::spawn(command, false)?;
    loop {
        if foreground_cancelled() {
            child.terminate();
            bail!("Command cancelled");
        }
        if let Some(status) = child
            .child
            .try_wait()
            .context("Failed to wait for command")?
        {
            if platform::was_interrupted(status) {
                cancel_foreground();
            }
            println!("Command exited: {status}");
            return Ok(CommandOutcome { status });
        }
        thread::sleep(POLL_INTERVAL);
    }
}

struct Drained {
    bytes: Vec<u8>,
    overflow: bool,
}

fn drain(mut reader: impl Read) -> io::Result<Drained> {
    let mut bytes = Vec::new();
    let mut overflow = false;
    let mut buffer = [0; 8192];
    loop {
        let count = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        let retain = count.min(OUTPUT_LIMIT - bytes.len());
        bytes.extend_from_slice(&buffer[..retain]);
        overflow |= retain < count;
    }
    Ok(Drained { bytes, overflow })
}

pub fn run_capture(command: &ResolvedCommand) -> Result<CapturedOutcome> {
    run_capture_with_timeout(command, CAPTURE_TIMEOUT)
}

fn run_capture_with_timeout(
    command: &ResolvedCommand,
    timeout: Duration,
) -> Result<CapturedOutcome> {
    let deadline = Instant::now() + timeout;
    let mut child = ManagedChild::spawn(command, true)?;
    let stdout = child.child.stdout.take().context("Missing stdout pipe")?;
    let stderr = child.child.stderr.take().context("Missing stderr pipe")?;
    let (sender, receiver) = mpsc::channel();
    let stdout_sender = sender.clone();
    let out_reader = thread::spawn(move || {
        let _ = stdout_sender.send((true, drain(stdout)));
    });
    let err_reader = thread::spawn(move || {
        let _ = sender.send((false, drain(stderr)));
    });
    let mut stdout = None;
    let mut stderr = None;
    let mut status = None;
    let result = (|| {
        loop {
            while let Ok((is_stdout, drained)) = receiver.try_recv() {
                let drained = drained.context("Failed to read command output")?;
                if is_stdout {
                    stdout = Some(drained);
                } else {
                    stderr = Some(drained);
                }
            }
            if status.is_none() {
                status = child
                    .child
                    .try_wait()
                    .context("Failed to wait for command")?;
            }
            if status.is_some() && stdout.is_some() && stderr.is_some() {
                return Ok(());
            }
            if SHUTDOWN.load(Ordering::SeqCst) {
                bail!("Command cancelled during application shutdown");
            }
            if Instant::now() >= deadline {
                bail!("Command timed out after {} ms", timeout.as_millis());
            }
            thread::sleep(POLL_INTERVAL.min(deadline.saturating_duration_since(Instant::now())));
        }
    })();
    // The deadline includes pipe EOF, even if the original process exited first.
    // Terminating the group closes pipes held by managed descendants before join.
    child.terminate();
    let _ = out_reader.join();
    let _ = err_reader.join();
    result?;
    let stdout = stdout.context("Missing stdout result")?;
    let stderr = stderr.context("Missing stderr result")?;
    Ok(CapturedOutcome {
        status: status.context("Missing command exit status")?,
        stdout: stdout.bytes,
        stderr: stderr.bytes,
        stdout_overflow: stdout.overflow,
        stderr_overflow: stderr.overflow,
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn shell(script: &str) -> ResolvedCommand {
        ResolvedCommand {
            program: "/bin/sh".into(),
            args: vec!["-c".into(), script.into()],
            cwd: std::env::temp_dir(),
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
    fn deadline_includes_pipes_held_after_parent_exits() {
        let started = Instant::now();
        let result =
            run_capture_with_timeout(&shell("sleep 30 & exit 0"), Duration::from_millis(150));
        assert!(result.unwrap_err().to_string().contains("timed out"));
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
                "(sleep 0.4; printf escaped > \"$1\") & exit 0".into(),
                "test".into(),
                marker.as_os_str().to_owned(),
            ],
            cwd: std::env::temp_dir(),
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
