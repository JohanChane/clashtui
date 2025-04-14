use std::process::{Command, Output};

use std::io::Result;

pub fn set_clash_permission(binary_path: &str) -> Result<String> {
    super::exec("chmod", vec!["+x", binary_path])?;
    exec_as_superuser(
        "setcap",
        vec!["'cap_net_admin,cap_net_bind_service=+ep'", binary_path],
    )
}

/// exec pgm via `pkexec`
pub fn exec_as_superuser(pgm: &str, args: Vec<&str>) -> Result<String> {
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
