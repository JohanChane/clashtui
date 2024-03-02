#[cfg(target_os = "windows")]
use encoding::{all::GBK, DecoderTrap, Encoding};
use std::process::{Command, Output, Stdio};

type Result<T> = core::result::Result<T, std::io::Error>;

pub fn exec(pgm: &str, args: Vec<&str>) -> Result<String> {
    log::debug!("IPC: {} {:?}", pgm, args);
    let output = Command::new(pgm).args(args).output()?;
    string_process_output(output)
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
#[cfg(target_os = "linux")]
pub fn exec_with_sbin(pgm: &str, args: Vec<&str>) -> Result<String> {
    log::debug!("LIPC: {} {:?}", pgm, args);
    let mut path = std::env::var("PATH").unwrap_or_default();
    path.push_str(":/usr/sbin");
    let output = Command::new(pgm).env("PATH", path).args(args).output()?;
    string_process_output(output)
}

#[cfg(target_os = "windows")]
fn execute_powershell_script(script: &str) -> Result<String> {
    string_process_output(
        Command::new("powershell")
            .arg("-Command")
            .arg(script)
            .output()?,
    )
}
#[cfg(target_os = "windows")]
pub fn start_process_as_admin(path: &str, arg_list: &str, does_wait: bool) -> Result<String> {
    let wait_op = if does_wait { "-Wait" } else { "" };
    let arg_op = if arg_list.is_empty() {
        String::new()
    } else {
        format!("-ArgumentList '{}'", arg_list)
    };

    let output = Command::new("powershell")
        .arg("-Command")
        .arg(&format!(
            "Start-Process {} -FilePath '{}' {} -Verb 'RunAs'",
            wait_op, path, arg_op
        ))
        .output()?;

    string_process_output(output)
}
#[cfg(target_os = "windows")]
pub fn execute_powershell_script_as_admin(cmd: &str, does_wait: bool) -> Result<String> {
    let wait_op = if does_wait { "-Wait" } else { "" };
    let cmd_op: String = if cmd.is_empty() {
        String::new()
    } else {
        format!("-ArgumentList '-Command {}'", cmd)
    };
    let output = Command::new("powershell")
        .arg("-Command")
        .arg(&format!(
            "Start-Process {} -FilePath powershell {} -Verb 'RunAs' 2>&1 | Out-String",
            wait_op, cmd_op
        ))
        .output()?;

    string_process_output(output)
}
#[cfg(target_os = "windows")]
pub fn enable_system_proxy(proxy_addr: &String) -> Result<String> {
    let enable_script = format!(
        r#"
        $proxyAddress = "{}"
        $regPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings"
        Set-ItemProperty -Path $regPath -Name ProxyEnable -Value 1
        Set-ItemProperty -Path $regPath -Name ProxyServer -Value $proxyAddress
        gpupdate /force
    "#,
        proxy_addr
    );

    execute_powershell_script(&enable_script)
}

#[cfg(target_os = "windows")]
pub fn disable_system_proxy() -> Result<String> {
    let disable_script = r#"
        $regPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings"
        Set-ItemProperty -Path $regPath -Name ProxyEnable -Value 0
        gpupdate /force
    "#;
    //Remove-ItemProperty -Path $regPath -Name ProxyServer

    execute_powershell_script(disable_script)
}

#[cfg(target_os = "windows")]
pub fn is_system_proxy_enabled() -> Result<bool> {
    let reg_query_output = Command::new("reg")
        .args(&[
            "query",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings",
            "/v",
            "ProxyEnable",
        ])
        .output()?;

    let output_str = String::from_utf8_lossy(&reg_query_output.stdout);

    //log::error!("{}", output_str);
    // Assuming the output format is like "ProxyEnable    REG_DWORD    0x00000001"
    let is_enabled = output_str.contains("REG_DWORD")
        && (output_str.contains("0x1") || output_str.contains("0x00000001"));

    Ok(is_enabled)
}

#[cfg(target_os = "windows")]
fn string_process_output(output: Output) -> Result<String> {
    let stdout_vec: Vec<u8> = output.stdout;
    use std::io::{Error, ErrorKind};

    let stdout_str = GBK
        .decode(&stdout_vec, DecoderTrap::Strict)
        .map_err(|err| {
            Error::new(
                ErrorKind::InvalidData,
                format!("Failed to decode stdout: {err}"),
            )
        })?;

    let stderr_str = GBK
        .decode(&output.stderr, DecoderTrap::Strict)
        .map_err(|err| {
            Error::new(
                ErrorKind::InvalidData,
                format!("Failed to decode stderr: {err}"),
            )
        })?;

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
#[cfg(target_os = "linux")]
fn string_process_output(output: Output) -> Result<String> {
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
