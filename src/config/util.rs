use anyhow::{Context, Result, bail};

pub mod defs {
    pub const CONFIG_FILE: &str = "config.yaml";
    pub const DATA_FILE: &str = "clashtui.db";
    pub const CORE_OVERRIDE_FILE: &str = "core_override_config.yaml";
    pub const CORE_OVERRIDE_SINGBOX_FILE: &str = "core_override_config.json";
    #[cfg(feature = "customized-theme")]
    pub const THEME_FILE: &str = "theme.yaml";
    pub const PROFILE_YAMLS_DIR: &str = "profiles";
    pub const PROFILE_JSONS_DIR: &str = "profiles";
    pub const TEMPLATE_DIR: &str = "templates";
    pub const KEYMAP_FILE: &str = "keymap.yaml";
    pub const PROXY_PROVIDERS_DIR: &str = "proxy-providers";
}

pub(super) fn load_home_dir() -> Result<std::path::PathBuf> {
    use std::{env, path};
    let data_dir = env::current_exe()
        .context("Err loading exe_file_path")?
        .parent()
        .context("Err finding exe_dir")?
        .join("data");
    if data_dir.exists() && data_dir.is_dir() {
        // portable mode
        Ok(data_dir)
    } else {
        if cfg!(target_os = "linux") {
            env::var_os("XDG_CONFIG_HOME")
                .map(path::PathBuf::from)
                .or(env::var_os("HOME").map(|h| path::PathBuf::from(h).join(".config")))
        } else if cfg!(target_os = "windows") {
            env::var_os("APPDATA").map(path::PathBuf::from)
        } else if cfg!(target_os = "macos") {
            env::var_os("HOME").map(|h| path::PathBuf::from(h).join(".config"))
        } else {
            bail!("Not supported platform")
        }
        .map(|c| c.join("clashtui"))
        .context("failed to load home dir")
    }
}

#[cfg(target_family = "unix")]
pub fn check_startup_perms() {
    use std::io::Write;

    let dirs_to_check = [
        &crate::config::CONFIG.cfg_file.mihomo.core.config_dir,
        &crate::config::CONFIG.cfg_file.singbox.core.config_dir,
    ];

    for dir_str in &dirs_to_check {
        if dir_str.is_empty() {
            continue;
        }
        let dir = std::path::Path::new(dir_str);
        if !dir.exists() {
            continue;
        }
        if has_write_access(dir) {
            continue;
        }

        print!(
            "File permissions in '{}' need repair. Fix now? [Y/n] ",
            dir.display()
        );
        let _ = std::io::stdout().flush();
        let mut input = String::new();
        let _ = std::io::stdin().read_line(&mut input);

        if input.trim().to_lowercase().as_str() != "y" {
            continue;
        }

        let Some(group) = get_dir_group_name(dir) else {
            continue;
        };

        if let Err(e) = (|| -> Result<()> {
            let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let _signals = crate::command::signal_scope(cancel.clone())?;
            repair_file_permissions(dir, &group, &cancel)
        })() {
            eprintln!("Error: {}", e);
            use std::io::Read;
            print!("Press Enter to continue...");
            let _ = std::io::stdout().flush();
            let _ = std::io::stdin().read(&mut [0u8]);
        }
    }
}

/// Check the effective user's real access, independent of the service scope.
#[cfg(target_family = "unix")]
fn has_write_access(path: &std::path::Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return false;
    };
    let symlink = metadata.is_symlink();
    let metadata = if symlink {
        let Ok(target) = std::fs::metadata(path) else {
            return false;
        };
        target
    } else {
        metadata
    };
    let Ok(name) = std::ffi::CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };
    let mode = libc::R_OK | libc::W_OK | if metadata.is_dir() { libc::X_OK } else { 0 };
    if unsafe { libc::faccessat(libc::AT_FDCWD, name.as_ptr(), mode, libc::AT_EACCESS) } != 0 {
        return false;
    }
    if metadata.is_dir() && !symlink {
        let Ok(entries) = std::fs::read_dir(path) else {
            return false;
        };
        entries
            .into_iter()
            .all(|entry| entry.is_ok_and(|entry| has_write_access(&entry.path())))
    } else {
        true
    }
}
#[cfg(not(target_family = "unix"))]
pub fn check_startup_perms() {}

macro_rules! load_save {
    ($id:ident, $name:expr) => {
        impl $id {
            pub fn to_file(&self) -> Result<()> {
                let path = DATA_DIR.get().unwrap().join($name);
                let fp = std::fs::File::create(&path)
                    .with_context(|| format!("Failed to create {}", path.display()))?;
                serde_yml::to_writer(fp, &self)
                    .with_context(|| format!("Failed to write {}", path.display()))
            }
        }
        load_save!($id, $name, no_save);
    };
    ($id:ident, $name:expr, no_save) => {
        impl $id {
            pub fn from_file() -> Result<Self> {
                let path = DATA_DIR.get().unwrap().join($name);
                let fp = std::fs::File::open(&path)
                    .with_context(|| format!("Failed to open {}", path.display()))?;
                serde_yml::from_reader(fp)
                    .with_context(|| format!("Failed to parse {}", path.display()))
            }
        }
    };
    ($id:ident, $name:expr, no_save, $subdir:expr) => {
        impl $id {
            pub fn from_file() -> Result<Self> {
                let path = DATA_DIR.get().unwrap().join($subdir).join($name);
                let fp = std::fs::File::open(&path)
                    .with_context(|| format!("Failed to open {}", path.display()))?;
                serde_yml::from_reader(fp)
                    .with_context(|| format!("Failed to parse {}", path.display()))
            }
        }
    };
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "macos")]
    #[test]
    fn load_home_dir_macos_uses_home_dot_config() {
        // When not in portable mode, macOS uses $HOME/.config/clashtui
        let exe_dir = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let portable_data = exe_dir.join("data");
        if portable_data.exists() && portable_data.is_dir() {
            eprintln!("skipping: portable data dir exists at {:?}", portable_data);
            return;
        }
        let result = super::load_home_dir().unwrap();
        assert!(
            result.ends_with(".config/clashtui"),
            "expected path ending with .config/clashtui, got: {result:?}"
        );
    }
}

#[cfg(unix)]
use {
    nix::unistd::{Gid, Group},
    std::{
        os::unix::fs::{MetadataExt, PermissionsExt},
        path::{Path, PathBuf},
    },
};
#[cfg(unix)]
fn find_files_not_group_writable(dir: &Path) -> Vec<PathBuf> {
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

#[cfg(unix)]
fn find_files_not_in_group(dir: &Path, group_name: &str) -> Vec<PathBuf> {
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

#[cfg(unix)]
fn get_dir_group_name(dir: &Path) -> Option<String> {
    let metadata = std::fs::metadata(dir).ok()?;
    Group::from_gid(Gid::from_raw(metadata.gid()))
        .ok()
        .flatten()
        .map(|g| g.name)
}

#[cfg(unix)]
fn repair_file_permissions(
    dir: &Path,
    group_name: &str,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<()> {
    fn sudo(args: Vec<std::ffi::OsString>, cancel: &std::sync::atomic::AtomicBool) -> Result<()> {
        let status = crate::command::inherit(std::ffi::OsStr::new("sudo"), &args, cancel)?;
        println!("Command exited: {status}");
        anyhow::ensure!(status.success(), "sudo failed: {status}");
        Ok(())
    }
    sudo(
        vec![
            "--".into(),
            "chmod".into(),
            "g+s".into(),
            "--".into(),
            dir.into(),
        ],
        cancel,
    )?;
    for path in find_files_not_in_group(dir, group_name) {
        sudo(
            vec![
                "--".into(),
                "chown".into(),
                format!(":{group_name}").into(),
                "--".into(),
                path.into(),
            ],
            cancel,
        )?;
    }
    for path in find_files_not_group_writable(dir) {
        sudo(
            vec![
                "--".into(),
                "chmod".into(),
                "g+w".into(),
                "--".into(),
                path.into(),
            ],
            cancel,
        )?;
    }
    Ok(())
}
#[cfg(all(test, unix))]
mod permission_tests {
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
