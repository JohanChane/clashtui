use super::platform::stringify_output;
use anyhow::Result;
use std::process::{Command, Stdio};

const COMMAND_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
const OUTPUT_LIMIT: usize = 2 * 1024 * 1024;

pub(crate) fn probe(
    command: &mut Command,
    timeout: std::time::Duration,
) -> Result<std::process::Output> {
    run_captured(command, false, timeout, crate::tui::command_cancelled)
}

pub fn exec(pgm: &str, args: Vec<&str>) -> Result<String> {
    log::debug!("IPC: {} {:?}", pgm, args);
    let output = run_captured(
        Command::new(pgm).args(args),
        false,
        COMMAND_TIMEOUT,
        crate::tui::command_cancelled,
    )?;
    Ok(stringify_output(output))
}

pub fn exec_sudo(pgm: &str, args: Vec<&str>) -> Result<String> {
    log::debug!("IPC: sudo {:?}", args);
    let lease = crate::tui::terminal_lease()?;
    let output = run_captured(
        Command::new("sudo").arg(pgm).args(args),
        true,
        COMMAND_TIMEOUT,
        || lease.is_cancelled(),
    )?;
    lease.release()?;
    Ok(stringify_output(output))
}

struct Captured {
    bytes: Vec<u8>,
    truncated: bool,
}

trait CapturedPipe: std::io::Read + Send + 'static {
    #[cfg(windows)]
    fn handle(&self) -> std::os::windows::io::RawHandle;
}

impl CapturedPipe for std::process::ChildStdout {
    #[cfg(windows)]
    fn handle(&self) -> std::os::windows::io::RawHandle {
        std::os::windows::io::AsRawHandle::as_raw_handle(self)
    }
}

impl CapturedPipe for std::process::ChildStderr {
    #[cfg(windows)]
    fn handle(&self) -> std::os::windows::io::RawHandle {
        std::os::windows::io::AsRawHandle::as_raw_handle(self)
    }
}

#[cfg(windows)]
fn available_bytes(pipe: &impl CapturedPipe) -> std::io::Result<Option<usize>> {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn PeekNamedPipe(
            pipe: *mut std::ffi::c_void,
            buffer: *mut std::ffi::c_void,
            size: u32,
            read: *mut u32,
            available: *mut u32,
            left: *mut u32,
        ) -> i32;
    }
    let mut available = 0;
    let ok = unsafe {
        PeekNamedPipe(
            pipe.handle(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            &mut available,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(109) {
            return Ok(None);
        } // ERROR_BROKEN_PIPE
        return Err(error);
    }
    Ok(Some(available as usize))
}

fn capture_pipe(
    mut pipe: impl CapturedPipe,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    echo: bool,
) -> std::thread::JoinHandle<std::io::Result<Captured>> {
    std::thread::spawn(move || {
        use std::io::Write;
        use std::sync::atomic::Ordering;
        let mut bytes = Vec::new();
        let mut truncated = false;
        let mut buffer = [0; 8192];
        loop {
            #[cfg(windows)]
            let limit = match available_bytes(&pipe)? {
                None => break,
                Some(0) if stop.load(Ordering::Acquire) => break,
                Some(0) => {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    continue;
                }
                Some(available) => available.min(buffer.len()),
            };
            #[cfg(not(windows))]
            let limit = buffer.len();
            match pipe.read(&mut buffer[..limit]) {
                Ok(0) => break,
                Ok(count) => {
                    let keep = count.min(OUTPUT_LIMIT.saturating_sub(bytes.len()));
                    bytes.extend_from_slice(&buffer[..keep]);
                    truncated |= keep < count;
                    if echo {
                        // Known Unix limitation: with inherited TOSTOP, forwarding stderr
                        // after lending the foreground to the child can stop clashtui via
                        // SIGTTOU. The tcsetpgrp signal mask does not cover this write.
                        let mut stderr = std::io::stderr().lock();
                        stderr.write_all(&buffer[..count])?;
                        stderr.flush()?;
                    }
                    if stop.load(Ordering::Acquire) && truncated {
                        break;
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if stop.load(Ordering::Acquire) {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Err(error) => return Err(error),
            }
        }
        Ok(Captured { bytes, truncated })
    })
}

#[cfg(unix)]
fn nonblocking(pipe: &impl std::os::fd::AsRawFd) -> std::io::Result<()> {
    let fd = pipe.as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(unix)]
struct ForegroundGroup {
    terminal: std::fs::File,
    previous: libc::pid_t,
}

#[cfg(unix)]
fn set_foreground(fd: std::os::fd::RawFd, group: libc::pid_t) -> std::io::Result<()> {
    // Restoring the App's foreground group happens while its worker is in the
    // background. Block SIGTTOU only on this thread during that operation.
    unsafe {
        let mut blocked = std::mem::zeroed();
        let mut previous = std::mem::zeroed();
        libc::sigemptyset(&mut blocked);
        libc::sigaddset(&mut blocked, libc::SIGTTOU);
        let error = libc::pthread_sigmask(libc::SIG_BLOCK, &blocked, &mut previous);
        if error != 0 {
            return Err(std::io::Error::from_raw_os_error(error));
        }
        let result = libc::tcsetpgrp(fd, group);
        let error = std::io::Error::last_os_error();
        libc::pthread_sigmask(libc::SIG_SETMASK, &previous, std::ptr::null_mut());
        if result < 0 { Err(error) } else { Ok(()) }
    }
}

#[cfg(unix)]
impl ForegroundGroup {
    fn lend(group: libc::pid_t) -> std::io::Result<Option<Self>> {
        use std::os::fd::AsRawFd;
        let terminal = match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")
        {
            Ok(terminal) => terminal,
            Err(error)
                if matches!(
                    error.raw_os_error(),
                    Some(libc::ENXIO | libc::ENODEV | libc::ENOENT)
                ) =>
            {
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        let previous = unsafe { libc::tcgetpgrp(terminal.as_raw_fd()) };
        if previous < 0 {
            return Err(std::io::Error::last_os_error());
        }
        set_foreground(terminal.as_raw_fd(), group)?;
        // A fast child may have tried to read before tcsetpgrp and received SIGTTIN.
        unsafe {
            libc::kill(-group, libc::SIGCONT);
        }
        Ok(Some(Self { terminal, previous }))
    }
}

#[cfg(unix)]
impl Drop for ForegroundGroup {
    fn drop(&mut self) {
        use std::os::fd::AsRawFd;
        if let Err(error) = set_foreground(self.terminal.as_raw_fd(), self.previous) {
            log::warn!("Restore terminal foreground group: {error}");
        }
    }
}

fn terminate_child(child: &mut std::process::Child) -> std::io::Result<std::process::ExitStatus> {
    #[cfg(unix)]
    unsafe {
        // Only this command's process group is managed; detached daemons are not included.
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/T", "/F", "/PID", &child.id().to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    child.wait()
}

fn run_captured(
    command: &mut Command,
    echo_stderr: bool,
    timeout: std::time::Duration,
    cancelled: impl Fn() -> bool,
) -> Result<std::process::Output> {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use std::time::{Duration, Instant};
    anyhow::ensure!(!cancelled(), "Command cancelled");
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn()?;
    #[cfg(unix)]
    let _foreground = if echo_stderr {
        match ForegroundGroup::lend(child.id() as libc::pid_t) {
            Ok(guard) => guard,
            Err(error) => {
                let _ = terminate_child(&mut child);
                return Err(error.into());
            }
        }
    } else {
        None
    };
    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");
    #[cfg(unix)]
    if let Err(error) = nonblocking(&stdout).and_then(|_| nonblocking(&stderr)) {
        let _ = terminate_child(&mut child);
        return Err(error.into());
    }
    let stop = Arc::new(AtomicBool::new(false));
    let stdout = capture_pipe(stdout, stop.clone(), false);
    let stderr = capture_pipe(stderr, stop.clone(), echo_stderr);
    let started = Instant::now();
    let mut failure = None;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => {}
            Err(error) => {
                let _ = terminate_child(&mut child);
                break Err(error);
            }
        }
        if cancelled() || started.elapsed() >= timeout {
            failure = Some(if cancelled() {
                "Command cancelled"
            } else {
                "Command timed out"
            });
            break terminate_child(&mut child);
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    // Unix readers drain bytes already available, then stop even if a descendant
    // inherited a pipe. This does not pretend that detached descendants were killed.
    stop.store(true, Ordering::Release);
    let stdout = stdout.join();
    let stderr = stderr.join();
    let stdout = stdout.map_err(|_| anyhow::anyhow!("stdout reader panicked"))?;
    let stderr = stderr.map_err(|_| anyhow::anyhow!("stderr reader panicked"))?;
    let stdout = stdout?;
    let stderr = stderr?;
    let output = std::process::Output {
        status: status?,
        stdout: stdout.bytes,
        stderr: stderr.bytes,
    };
    if let Some(reason) = failure {
        anyhow::bail!("{reason}\n{}", stringify_output(output));
    }
    anyhow::ensure!(
        !stdout.truncated && !stderr.truncated,
        "Command output exceeded {OUTPUT_LIMIT} bytes per stream"
    );
    Ok(output)
}

pub fn spawn(pgm: &str, args: Vec<&str>) -> Result<()> {
    log::debug!("SPW: {} {:?}", pgm, args);
    Command::new(pgm)
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .args(args)
        .spawn()?;
    Ok(())
}

fn sanitize_windows_path(path: &str) -> String {
    let path = path.strip_prefix(r"\\?\").unwrap_or(path);
    path.replace('\\', "/")
}

pub fn shell_spawn(cmd_template: &str, path: &str) -> Result<()> {
    if cmd_template.is_empty() {
        if cfg!(windows) {
            let path = sanitize_windows_path(path);
            spawn("cmd", vec!["/c", "start", "", &path])
        } else if cfg!(target_os = "macos") {
            spawn("sh", vec!["-c", &format!("open \"{}\"", path)])
        } else {
            spawn("sh", vec!["-c", &format!("xdg-open \"{}\"", path)])
        }
    } else {
        if cfg!(windows) {
            let path = sanitize_windows_path(path);
            let cmd = cmd_template.replace("%s", &path);
            log::debug!("SPW: cmd {} {}", "cmd", cmd);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                Command::new("cmd")
                    .stderr(Stdio::null())
                    .stdout(Stdio::null())
                    .raw_arg("/c")
                    .raw_arg(&cmd)
                    .spawn()?;
            }
            Ok(())
        } else {
            let cmd = cmd_template.replace("%s", path);
            spawn("sh", vec!["-c", &cmd])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn command_drains_both_pipes_without_deadlock() {
        let output = run_captured(
            Command::new("sh").args([
                "-c",
                "head -c 200000 /dev/zero; head -c 200000 /dev/zero >&2",
            ]),
            false,
            std::time::Duration::from_secs(5),
            || false,
        )
        .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout.len(), 200000);
        assert_eq!(output.stderr.len(), 200000);
    }

    #[cfg(unix)]
    #[test]
    fn timeout_reaps_child_and_pipe_readers() {
        let started = std::time::Instant::now();
        let error = run_captured(
            Command::new("sh").args(["-c", "sleep 10 & wait"]),
            false,
            std::time::Duration::from_millis(30),
            || false,
        )
        .unwrap_err();
        assert!(error.to_string().contains("Command timed out"));
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
    }

    #[cfg(unix)]
    #[test]
    fn cancellation_reaps_child_and_preserves_partial_output() {
        let started = std::time::Instant::now();
        let error = run_captured(
            Command::new("sh").args(["-c", "printf before-cancel; sleep 10 & wait"]),
            false,
            std::time::Duration::from_secs(5),
            || started.elapsed() >= std::time::Duration::from_millis(50),
        )
        .unwrap_err();
        assert!(error.to_string().contains("Command cancelled"));
        assert!(error.to_string().contains("before-cancel"));
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
    }

    #[test]
    fn sanitize_unc_prefix_stripped() {
        assert_eq!(sanitize_windows_path(r"\\?\C:\Users\foo"), "C:/Users/foo");
    }

    #[test]
    fn sanitize_non_unc_untouched() {
        assert_eq!(sanitize_windows_path(r"C:\Users\foo"), "C:/Users/foo");
    }

    #[test]
    fn sanitize_forward_slashes_unchanged() {
        assert_eq!(sanitize_windows_path("C:/Users/foo"), "C:/Users/foo");
    }

    #[test]
    fn sanitize_mixed_slashes_converted() {
        assert_eq!(sanitize_windows_path(r"C:\foo/bar\baz"), "C:/foo/bar/baz");
    }

    #[test]
    fn sanitize_unc_with_mixed_slashes() {
        assert_eq!(
            sanitize_windows_path(r"\\?\C:\foo/bar\baz"),
            "C:/foo/bar/baz"
        );
    }

    #[test]
    fn sanitize_empty_string() {
        assert_eq!(sanitize_windows_path(""), "");
    }

    #[test]
    fn sanitize_path_without_backslashes() {
        assert_eq!(sanitize_windows_path("C:/foo/bar/baz"), "C:/foo/bar/baz");
    }
}
