use super::*;
#[cfg(unix)]
use std::path::PathBuf;

// ============================================================================
// Platform stubs — file permissions / TUN capability (no-ops on Windows)
// ============================================================================

#[cfg(unix)]
pub fn correct_cap_for_tun() -> Result<String> {
    Ok("No setcap on Windows".into())
}

#[cfg(unix)]
pub fn find_files_not_group_writable(_dir: &Path) -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(unix)]
pub fn find_files_not_in_group(_dir: &Path, _group_name: &str) -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(unix)]
pub fn get_dir_group_name(_dir: &Path) -> Option<String> {
    None
}

#[cfg(unix)]
pub fn check_file_permissions(_dir: &Path) -> bool {
    true
}

#[cfg(unix)]
pub fn repair_file_permissions(_dir: &Path, _group_name: &str) -> Result<String> {
    Ok("Permissions OK on Windows".into())
}

pub(super) fn stringify_output(output: std::process::Output) -> String {
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);

    format!(
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
    )
}
