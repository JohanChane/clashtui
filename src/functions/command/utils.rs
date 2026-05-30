use super::platform::stringify_output;
use anyhow::Result;
use std::{
    io::Write,
    process::{Command, Stdio},
};

pub fn exec(pgm: &str, args: Vec<&str>) -> Result<String> {
    log::debug!("IPC: {} {:?}", pgm, args);
    let output = Command::new(pgm).args(args).output()?;
    Ok(stringify_output(output))
}

pub fn exec_sudo(args: Vec<&str>, password: &str) -> Result<String> {
    log::debug!("IPC: sudo -S {:?}", args);
    let mut cmd = Command::new("sudo");
    cmd.arg("-S");
    cmd.args(args);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(password.as_bytes())?;
        stdin.write_all(b"\n")?;
    }
    let output = child.wait_with_output()?;
    Ok(stringify_output(output))
}
pub fn sudo_needs_password() -> bool {
    !std::process::Command::new("sudo")
        .args(["-n", "true"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
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
    let path = path
        .strip_prefix(r"\\?\")
        .unwrap_or(path);
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

    #[test]
    fn sanitize_unc_prefix_stripped() {
        assert_eq!(
            sanitize_windows_path(r"\\?\C:\Users\foo"),
            "C:/Users/foo"
        );
    }

    #[test]
    fn sanitize_non_unc_untouched() {
        assert_eq!(
            sanitize_windows_path(r"C:\Users\foo"),
            "C:/Users/foo"
        );
    }

    #[test]
    fn sanitize_forward_slashes_unchanged() {
        assert_eq!(
            sanitize_windows_path("C:/Users/foo"),
            "C:/Users/foo"
        );
    }

    #[test]
    fn sanitize_mixed_slashes_converted() {
        assert_eq!(
            sanitize_windows_path(r"C:\foo/bar\baz"),
            "C:/foo/bar/baz"
        );
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
        assert_eq!(
            sanitize_windows_path("C:/foo/bar/baz"),
            "C:/foo/bar/baz"
        );
    }
}
