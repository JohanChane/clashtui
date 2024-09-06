use std::process::{Command, Output};

use std::io::Result;

/// exec pgm via `pkexec`
pub fn exec_with_sbin(pgm: &str, args: Vec<&str>) -> Result<String> {
    log::debug!("LIPC: {} {:?}", pgm, args);
    let mut execs = format!("pkexec {pgm}");
    execs.extend(args.into_iter().map(|s| format!(" {s}")));
    log::debug!("LIPC: {:?}", execs);
    let mut path = std::env::var("PATH").unwrap_or_default();
    path.push_str(":/usr/sbin");
    let output = Command::new("sh")
        .env("PATH", path)
        .arg("-c")
        .arg(execs)
        .output()?;
    string_process_output(output)
}

pub(super) fn string_process_output(output: Output) -> Result<String> {
    let stdout_str = String::from_utf8(output.stdout).unwrap();
    let stderr_str = String::from_utf8(output.stderr).unwrap();

    let result_str = format!(
        r#"
        Status:
        {}

        Stdout:
        {}

        Stderr:
        {}
        "#,
        output.status, stdout_str, stderr_str
    );

    Ok(result_str)
}
