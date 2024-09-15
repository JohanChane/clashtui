use std::process::{Command, Output};

use std::io::Result;

fn execute_powershell_script(script: &str) -> Result<String> {
    string_process_output(
        Command::new("powershell")
            .arg("-Command")
            .arg(script)
            .output()?,
    )
}
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
pub fn enable_system_proxy(proxy_addr: &str) -> Result<String> {
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

pub fn disable_system_proxy() -> Result<String> {
    let disable_script = r#"
        $regPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings"
        Set-ItemProperty -Path $regPath -Name ProxyEnable -Value 0
        gpupdate /force
    "#;
    //Remove-ItemProperty -Path $regPath -Name ProxyServer

    execute_powershell_script(disable_script)
}

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

pub(super) fn string_process_output(output: Output) -> Result<String> {
    let stdout_vec: Vec<u8> = output.stdout;

    let (stdout_str, _encoding, _contain_bad_char) =
        encoding_rs::Encoding::decode(encoding_rs::GBK, &stdout_vec);

    let (stderr_str, _encoding, _contain_bad_char) =
        encoding_rs::Encoding::decode(encoding_rs::GBK, &stdout_vec);

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
