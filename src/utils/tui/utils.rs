use std::process::Output;

pub fn concat_update_profile_result(
    result: (Vec<(String, String)>, Vec<(String, String)>),
) -> Vec<String> {
    let (updated_res, not_updated_res) = result;
    let mut concatenated_result = Vec::new();

    for (url, path) in not_updated_res {
        concatenated_result.push(format!("Not Updated: {} -> {}", url, path));
    }

    for (url, path) in updated_res {
        concatenated_result.push(format!("Updated: {} -> {}", url, path));
    }

    concatenated_result
}

pub fn get_file_names(dir: &std::path::Path) -> anyhow::Result<Vec<String>> {
    let mut file_names: Vec<String> = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name() {
                if let Some(file_name_str) = file_name.to_str() {
                    file_names.push(file_name_str.to_string());
                }
            }
        }
    }

    Ok(file_names)
}

#[cfg(target_os = "windows")]
fn execute_powershell_script(script: &str) -> Result<std::process::Output> {
    let output = Command::new("powershell")
        .arg("-Command")
        .arg(script)
        .output()?;

    return Ok(output);
}
#[cfg(target_os = "windows")]
pub fn start_process_as_admin(
    path: &str,
    arg_list: &str,
    does_wait: bool,
) -> Result<std::process::Output> {
    let wait_op = if does_wait { "-Wait" } else { "" };
    let arg_op = if arg_list.is_empty() {
        "".to_string()
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

    return Ok(output);
}
#[cfg(target_os = "windows")]
pub fn execute_powershell_script_as_admin(
    cmd: &str,
    does_wait: bool,
) -> Result<std::process::Output> {
    let wait_op = if does_wait { "-Wait" } else { "" };
    let cmd_op: String = if cmd.is_empty() {
        "".to_string()
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

    return Ok(output);
}
#[cfg(target_os = "windows")]
pub fn enable_system_proxy(proxy_addr: &String) -> Result<std::process::Output> {
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

    execute_powershell_script(&enable_script).context("Failed to enable system proxy")
}

#[cfg(target_os = "windows")]
pub fn disable_system_proxy() -> Result<std::process::Output> {
    let disable_script = r#"
        $regPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings"
        Set-ItemProperty -Path $regPath -Name ProxyEnable -Value 0
        gpupdate /force
    "#;
    //Remove-ItemProperty -Path $regPath -Name ProxyServer

    execute_powershell_script(disable_script).context("Failed to disable system proxy")
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

    let stdout_str = GBK
        .decode(&stdout_vec, DecoderTrap::Strict)
        .map_err(|err| anyhow!("Failed to decode stdout: {}", err))?;

    let stderr_str = GBK
        .decode(&output.stderr, DecoderTrap::Strict)
        .map_err(|err| anyhow!("Failed to decode stderr: {}", err))?;

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
pub fn string_process_output(output: Output) -> Result<String, std::io::Error> {
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
