use super::*;
use nix::unistd::{Gid, Group};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::PathBuf;

pub fn find_files_not_group_writable(dir: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                result.extend(find_files_not_group_writable(&path));
            }
            if let Ok(metadata) = entry.metadata()
                && metadata.permissions().mode() & 0o0020 == 0
            {
                result.push(path);
            }
        }
    }

    if let Ok(metadata) = std::fs::metadata(dir)
        && metadata.permissions().mode() & 0o0020 == 0
    {
        result.push(dir.to_path_buf());
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
            if let Ok(metadata) = entry.metadata()
                && let Some(group) = Group::from_gid(Gid::from_raw(metadata.gid()))
                    .ok()
                    .flatten()
                && group.name != group_name
            {
                result.push(path);
            }
        }
    }

    if let Ok(metadata) = std::fs::metadata(dir)
        && let Some(group) = Group::from_gid(Gid::from_raw(metadata.gid()))
            .ok()
            .flatten()
        && group.name != group_name
    {
        result.push(dir.to_path_buf());
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
    exec_sudo("sh", vec!["-c", &script])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn make_temp_dir() -> (PathBuf, impl Drop) {
        let path = std::env::temp_dir().join(format!(
            "clashtui_test_{}_{}",
            std::process::id(),
            fastrand::u64(..)
        ));
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
    fn test_find_files_not_group_writable_all_writable() {
        let (dir, _cleanup) = make_temp_dir();
        let file = dir.join("foo.txt");
        std::fs::write(&file, "bar").unwrap();
        let mut perms = std::fs::metadata(&file).unwrap().permissions();
        perms.set_mode(0o0660);
        std::fs::set_permissions(&file, perms).unwrap();

        let result = find_files_not_group_writable(&dir);
        // The file with group-write should NOT be in the result
        assert!(
            !result.contains(&file),
            "file with group-write should not be in result: {result:?}"
        );
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
        assert!(
            result.contains(&file1),
            "file1 should be in result: {result:?}"
        );
        assert!(
            result.contains(&file2),
            "file2 should be in result: {result:?}"
        );
    }
}
