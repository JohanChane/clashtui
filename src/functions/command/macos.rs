use super::*;
use std::path::Path;

pub fn correct_cap_for_tun() -> Result<String> {
    // macOS TUN works via utun devices; no setcap needed.
    // The core binary just needs to run as root for TUN access.
    Ok("No setcap on macOS (use sudo for TUN)".into())
}

pub fn find_files_not_group_writable(_dir: &Path) -> Vec<std::path::PathBuf> {
    Vec::new()
}

pub fn find_files_not_in_group(_dir: &Path, _group_name: &str) -> Vec<std::path::PathBuf> {
    Vec::new()
}

pub fn get_dir_group_name(_dir: &Path) -> Option<String> {
    None
}

pub fn check_file_permissions(_dir: &Path) -> bool {
    true
}

pub fn repair_file_permissions(_dir: &Path, _group_name: &str) -> Result<String> {
    Ok("File permission repair not needed on macOS".into())
}

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
