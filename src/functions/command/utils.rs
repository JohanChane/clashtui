use super::platform::stringify_output;
use anyhow::Result;
use std::process::{Command, Stdio};

pub fn exec(pgm: &str, args: Vec<&str>) -> Result<String> {
    log::debug!("IPC: {} {:?}", pgm, args);
    let output = Command::new(pgm).args(args).output()?;
    Ok(stringify_output(output))
}

pub fn exec_sudo(pgm: &str, args: Vec<&str>) -> Result<String> {
    use crate::tui;
    log::debug!("IPC: sudo -S {:?}", args);
    tui::hold(true)?;
    let mut child = Command::new("sudo")
        .arg(pgm)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut childstderr = child.stderr.take().unwrap();
    let mut stderr = std::io::stderr();
    let mut output_copy = Vec::new();
    let mut buffer = [0; 1024];

    // Read in a loop until the pipe closes
    loop {
        use std::io::{Read, Write};
        let n = childstderr.read(&mut buffer)?;
        if n == 0 {
            break; // EOF
        }
        // Save to memory (keep a copy)
        output_copy.extend_from_slice(&buffer[..n]);
        // Write to terminal
        stderr.write_all(&buffer[..n])?;
        stderr.flush()?;
    }
    eprintln!();
    let mut output = child.wait_with_output()?;
    output.stderr = output_copy;

    tui::hold(false)?;
    Ok(stringify_output(output))
}

// #[cfg(unix)]
// fn check_sudo_password_required() -> Result<bool> {
//     Command::new("sudo")
//         .args(["-n", "true"])
//         .stdout(Stdio::null())
//         .stderr(Stdio::null())
//         .status()
//         .map(|staus| staus.success())
//         .map_err(|e| e.into())
// }

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
