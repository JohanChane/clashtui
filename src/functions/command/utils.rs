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
    // Just ignore the output, otherwise the ui might be broken
    Command::new(pgm)
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .args(args)
        .spawn()?;
    Ok(())
}
