use anyhow::Result;
use std::process::{Command, Stdio};

pub fn exec_sudo(pgm: &str, args: Vec<&str>) -> Result<String> {
    let mut argv = vec!["--".into(), pgm.into()];
    argv.extend(args.into_iter().map(Into::into));
    let command = crate::config::scripts::ResolvedCommand {
        program: "sudo".into(),
        args: argv,
        cwd: std::env::current_dir()?,
    };
    crate::tui::with_terminal(|| {
        let outcome = crate::functions::process::run_foreground(&command)?;
        anyhow::ensure!(outcome.status.success(), "sudo failed: {}", outcome.status);
        Ok("OK".into())
    })
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
