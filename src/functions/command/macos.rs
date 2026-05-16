use super::*;
use nix::unistd::{Gid, Group};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::PathBuf;

pub fn correct_cap_for_tun() -> Result<String> {
    // macOS TUN works via utun devices; no setcap needed.
    // The core binary just needs to run as root for TUN access.
    Ok("No setcap on macOS (use sudo for TUN)".into())
}

pub fn find_files_not_group_writable(dir: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                result.extend(find_files_not_group_writable(&path));
            }
            if let Ok(metadata) = entry.metadata() {
                if metadata.permissions().mode() & 0o0020 == 0 {
                    result.push(path);
                }
            }
        }
    }

    if let Ok(metadata) = std::fs::metadata(dir) {
        if metadata.permissions().mode() & 0o0020 == 0 {
            result.push(dir.to_path_buf());
        }
    }

    result
}

pub fn find_files_not_in_group(dir: &Path, group_name: &str) -> Vec<PathBuf> {
    let mut result = Vec::new();

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                result.extend(find_files_not_in_group(&path, group_name));
            }
            if let Ok(metadata) = entry.metadata() {
                if let Some(group) = Group::from_gid(Gid::from_raw(metadata.gid()))
                    .ok()
                    .flatten()
                {
                    if group.name != group_name {
                        result.push(path);
                    }
                }
            }
        }
    }

    if let Ok(metadata) = std::fs::metadata(dir) {
        if let Some(group) = Group::from_gid(Gid::from_raw(metadata.gid()))
            .ok()
            .flatten()
        {
            if group.name != group_name {
                result.push(dir.to_path_buf());
            }
        }
    }

    result
}

pub fn get_dir_group_name(dir: &Path) -> Option<String> {
    let metadata = std::fs::metadata(dir).ok()?;
    Group::from_gid(Gid::from_raw(metadata.gid()))
        .ok()
        .flatten()
        .map(|g| g.name)
}

pub fn check_file_permissions(dir: &Path) -> bool {
    let metadata = match std::fs::metadata(dir) {
        Ok(m) => m,
        Err(_) => return true,
    };

    if metadata.permissions().mode() & 0o2000 == 0 {
        return false;
    }

    let Some(group_name) = get_dir_group_name(dir) else {
        return false;
    };

    find_files_not_in_group(dir, &group_name).is_empty()
        && find_files_not_group_writable(dir).is_empty()
}

pub fn repair_file_permissions(dir: &Path, group_name: &str) -> Result<String> {
    let mut commands: Vec<String> = Vec::new();
    let dir_str = dir.display().to_string();

    commands.push(format!("chmod g+s '{}'", dir_str));

    for file in &find_files_not_in_group(dir, group_name) {
        commands.push(format!("chown :{} '{}'", group_name, file.display()));
    }

    for file in &find_files_not_group_writable(dir) {
        commands.push(format!("chmod g+w '{}'", file.display()));
    }

    if commands.is_empty() {
        return Ok("Permissions OK, no repair needed".to_owned());
    }

    let script = commands.join(" && ");
    crate::tui::hold(true)?;
    let opt = std::process::Command::new("sudo")
        .arg("sh")
        .arg("-c")
        .arg(&script)
        .output()?;
    crate::tui::hold(false)?;
    Ok(stringify_output(opt))
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
