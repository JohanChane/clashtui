use std::io::Result;
use std::process::{Command, Stdio};

#[cfg_attr(target_os = "windows", path = "ipc/windows.rs")]
#[cfg_attr(target_os = "linux", path = "ipc/linux.rs")]
#[cfg_attr(target_os = "macos", path = "ipc/macos.rs")]
mod inner;

pub use inner::*;

pub fn exec(pgm: &str, args: Vec<&str>) -> Result<String> {
    log::debug!("IPC: {} {:?}", pgm, args);
    let output = Command::new(pgm).args(args).output()?;
    string_process_output(output)
}
#[allow(unused)]
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
