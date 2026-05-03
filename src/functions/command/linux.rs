use super::*;

pub fn correct_cap_for_tun() -> Result<String> {
    let binary_path = &crate::config::CONFIG.cfg_file.basic.clash_bin_path;

    exec("chmod", vec!["+x", binary_path])?;
    run_as_su_by_sudo(
        "setcap",
        &["'cap_net_admin,cap_net_bind_service=+ep'", binary_path],
    )
}

// fn check_sudo_password_required() -> Result<bool> {
//     Command::new("sudo")
//         .args(["-n", "true"])
//         .status()
//         .map(|staus| staus.success())
//         .map_err(|e| e.into())
// }

/// here we call `crate::tui::hold` to get back to normal screen,
/// leaving stdio to `sudo`, where user can enter their passwords
/// without worries
fn run_as_su_by_sudo(pgm: &str, args: &[&str]) -> Result<String> {
    crate::tui::hold(true)?;

    let opt = std::process::Command::new("sudo")
        // .arg("-S")
        .arg(pgm)
        .args(args)
        .output()?;

    crate::tui::hold(false)?;
    Ok(stringify_output(opt))
}

// fn run_as_su_by_pkexec(pgm: &str, args: &[&str]) -> Result<String> {
//     let mut path = std::env::var("PATH").unwrap_or_default();
//     path.push_str(":/usr/sbin");

//     let opt = Command::new("pkexec")
//         .env("PATH", path)
//         .arg(pgm)
//         .args(args)
//         .output()?;

//     Ok(stringify_output(opt))
// }

pub(super) fn stringify_output(output: std::process::Output) -> String {
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);

    let result_str = format!(
        r#"{}
        Stdout:
        {}

        Stderr:
        {}
        "#,
        if output.status.success() {
            "OK".to_owned()
        } else {
            format!("Error({})", output.status)
        },
        stdout_str,
        stderr_str
    );

    result_str
}
