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

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::process::ExitStatusExt;

    fn make_temp_dir() -> (PathBuf, impl Drop) {
        let uuid = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("clashtui_test_{uuid}"));
        std::fs::create_dir(&path).unwrap();
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        (path.clone(), Cleanup(path))
    }

    #[test]
    fn test_correct_cap_for_tun() {
        let result = correct_cap_for_tun().unwrap();
        assert!(result.contains("No setcap on macOS"));
        assert!(result.contains("sudo"));
    }

    #[test]
    fn test_stringify_output_success() {
        let output = std::process::Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: b"hello stdout\n"[..].into(),
            stderr: b"hello stderr\n"[..].into(),
        };
        let result = stringify_output(output);
        assert!(result.contains("OK"), "should contain OK for success: {result}");
        assert!(result.contains("hello stdout"));
        assert!(result.contains("hello stderr"));
    }

    #[test]
    fn test_stringify_output_failure() {
        let output = std::process::Output {
            status: std::process::ExitStatus::from_raw(1 << 8),
            stdout: b""[..].into(),
            stderr: b"something went wrong\n"[..].into(),
        };
        let result = stringify_output(output);
        assert!(result.contains("Error("), "should contain Error: {result}");
        assert!(result.contains("something went wrong"));
    }

    #[test]
    fn test_stringify_output_empty() {
        let output = std::process::Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: Vec::new(),
            stderr: Vec::new(),
        };
        let result = stringify_output(output);
        assert!(result.contains("OK"));
        assert!(result.contains("Stdout:"));
        assert!(result.contains("Stderr:"));
    }

    #[test]
    fn test_find_files_not_group_writable_all_writable() {
        let (dir, _cleanup) = make_temp_dir();
        let file = dir.join("foo.txt");
        std::fs::write(&file, "bar").unwrap();
        let mut perms = std::fs::metadata(&file).unwrap().permissions();
        perms.set_mode(0o0660);
        std::fs::set_permissions(&file, perms).unwrap();

        let result = find_files_not_group_writable(&dir);
        // The file with group-write should NOT be in the result
        assert!(!result.contains(&file), "file with group-write should not be in result: {result:?}");
    }

    #[test]
    fn test_find_files_not_group_writable_missing() {
        let (dir, _cleanup) = make_temp_dir();
        let file = dir.join("no_group_write.txt");
        std::fs::write(&file, "content").unwrap();
        let mut perms = std::fs::metadata(&file).unwrap().permissions();
        perms.set_mode(0o0644);
        std::fs::set_permissions(&file, perms).unwrap();

        let result = find_files_not_group_writable(&dir);
        assert!(result.contains(&file), "file without group-write should be in result: {result:?}");
    }

    #[test]
    fn test_find_files_not_group_writable_nested() {
        let (dir, _cleanup) = make_temp_dir();

        let sub = dir.join("sub");
        std::fs::create_dir(&sub).unwrap();
        let file1 = dir.join("a.txt");
        let file2 = sub.join("b.txt");
        std::fs::write(&file1, "a").unwrap();
        std::fs::write(&file2, "b").unwrap();

        // Only set permissions on files (not dirs) to avoid PermissionDenied on macOS
        for p in [&file1, &file2] {
            let mut perms = std::fs::metadata(p).unwrap().permissions();
            perms.set_mode(0o0644);
            std::fs::set_permissions(p, perms).unwrap();
        }

        let result = find_files_not_group_writable(&dir);
        assert!(result.contains(&file1), "file1 should be in result: {result:?}");
        assert!(result.contains(&file2), "file2 should be in result: {result:?}");
    }
}
