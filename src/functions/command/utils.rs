use super::platform::stringify_output;
use anyhow::Result;
use std::process::{Command, Stdio};

pub fn exec(pgm: &str, args: Vec<&str>) -> Result<String> {
    log::debug!("IPC: {} {:?}", pgm, args);
    let output = Command::new(pgm).args(args).output()?;
    Ok(stringify_output(output))
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
