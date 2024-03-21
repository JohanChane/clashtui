use std::{fs, process, env};
use std::os::unix::fs::{PermissionsExt, MetadataExt};
use nix::unistd::{Uid, Gid, Group, geteuid, setfsuid, setfsgid, initgroups};
use std::path::PathBuf;
use std::ffi::CString;
use std::os::unix::process::CommandExt;

pub(super) fn get_file_names<P>(dir: P) -> std::io::Result<Vec<String>>
where
    P: AsRef<std::path::Path>,
{
    let mut file_names: Vec<String> = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name() {
                file_names.push(file_name.to_string_lossy().to_string());
            }
        }
    }
    Ok(file_names)
}
/// Judging by format
pub(super) fn is_yaml(path: &std::path::Path) -> bool {
    std::fs::File::open(path).is_ok_and(|f| {
        serde_yaml::from_reader::<std::fs::File, serde_yaml::Value>(f).is_ok_and(|v| v.is_mapping())
    })
}

pub(super) fn parse_yaml(yaml_path: &std::path::Path) -> std::io::Result<serde_yaml::Value> {
    serde_yaml::from_reader(std::fs::File::open(yaml_path)?)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

pub fn get_mtime<P>(file_path: P) -> std::io::Result<std::time::SystemTime>
where
    P: AsRef<std::path::Path>,
{
    let file = std::fs::metadata(file_path)?;
    if file.is_file() {
        file.modified()
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Not a file?",
        ))
    }
}

pub fn str_duration(t: std::time::Duration) -> String {
    use std::time::Duration;
    if t.is_zero() {
        "Just Now".to_string()
    } else if t < Duration::from_secs(60 * 59) {
        let min = t.as_secs() / 60;
        format!("{}m", min + 1)
    } else if t < Duration::from_secs(3600 * 24) {
        let hou = t.as_secs() / 3600;
        format!("{hou}h")
    } else {
        let day = t.as_secs() / (3600 * 24);
        format!("{day}d")
    }
}

pub fn modify_file_perms_in_dir(dir: &PathBuf, group_name: &str) {
    // dir add set-group-id: `chmod g+s dir`
    if let Ok(metadata) = std::fs::metadata(dir) {
        let permissions = metadata.permissions();
        if permissions.mode() & 0o2000 == 0 {
            if let Ok(metadata) = fs::metadata(dir) {
                let permissions = metadata.permissions();
                let mut new_permissions = permissions.clone();
                new_permissions.set_mode(permissions.mode() | 0o2020);
                println!("Adding `g+s` permission to '{:?}'", dir);
                if let Err(e) = fs::set_permissions(dir, new_permissions) {
                    eprintln!("Failed to set `g+s` permissions for '{:?}': {}", dir, e);
                }
            }
        }
    }

    let files_not_in_group = find_files_not_in_group(dir, group_name);
    for file in &files_not_in_group {
        let path = std::path::Path::new(dir).join(file);
        if let Ok(group) = Group::from_name(group_name) {
            println!("Changing group to '{}' for {:?}:", group_name, file);
            if let Err(e) = nix::unistd::chown(&path, None, group.map(|g| g.gid)) {
                eprintln!("Failed to change group to '{}' for '{:?}': {}", group_name, file, e);
            }
        }
    }

    let files_not_group_writable = find_files_not_group_writable(dir);
    for file in &files_not_group_writable {
        if let Ok(metadata) = fs::metadata(file) {
            let permissions = metadata.permissions();
            let mut new_permissions = permissions.clone();
            new_permissions.set_mode(permissions.mode() | 0o0020);
            println!("Adding `g+w` permission to '{:?}'", file);
            if let Err(e) = fs::set_permissions(file, new_permissions) {
                eprintln!("Failed to set `g+w` permissions for '{:?}': {}", file, e);
            }
        }
    }
}

// Check dir member and dir itself.
pub fn find_files_not_group_writable(dir: &PathBuf) -> Vec<PathBuf> {
    let mut result = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                let metadata = entry.metadata().unwrap();
                if metadata.is_file() {
                    let permissions = metadata.permissions();

                    if permissions.mode() & 0o0020 == 0 {
                        result.push(path.clone());
                    }
                }
                if metadata.is_dir() {
                    result.extend(find_files_not_group_writable(&path));
                }
            }
        }
    }

    if let Ok(metadata) = fs::metadata(dir) {
        let permissions = metadata.permissions();
        if permissions.mode() & 0o0020 == 0 {
            result.push(dir.clone());
        }
    }

    result
}

// Check dir member and dir itself.
pub fn find_files_not_in_group(dir: &PathBuf, group_name: &str) -> Vec<PathBuf> {
    let mut result = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let metadata = entry.metadata().unwrap();

                if metadata.is_file() {
                    let file_gid = metadata.gid();
                    if let Ok(Some(group)) =
                        Group::from_gid(Gid::from_raw(file_gid))
                    {
                        if group.name != group_name {
                            result.push(entry.path().clone());
                        }
                    }
                } else if metadata.is_dir() {
                    let sub_dir = entry.path();
                    result.extend(find_files_not_in_group(&sub_dir, group_name));
                }
            }
        }
    }

    if let Ok(metadata) = fs::metadata(dir) {
        if let Some(dir_group) =
            Group::from_gid(Gid::from_raw(metadata.gid())).unwrap()
        {
            if dir_group.name != group_name {
                result.push(dir.clone());
            }
        }
    }

    result
}

pub fn get_file_group_name(dir: &PathBuf) -> Option<String> {
    if let Ok(metadata) = std::fs::metadata(dir) {
        if let Some(dir_group) = 
            nix::unistd::Group::from_gid(nix::unistd::Gid::from_raw(metadata.gid())).unwrap()
        {
            return Some(dir_group.name);
        }
    }

    None
}

// Perform file operations with clashtui process as a sudo user.
pub fn mock_fileop_as_sudo_user() {
    if ! is_run_as_root() {
        return;
    }

    // sudo printenv: SUDO_USER, SUDO_UID, SUDO_GID, ...
    if let (Ok(uid_str), Ok(gid_str)) = (env::var("SUDO_UID"), env::var("SUDO_GID")) {
        if let (Ok(uid_num), Ok(gid_num)) = (uid_str.parse::<u32>(), gid_str.parse::<u32>()) {
            // In Linux, file operation permissions are determined using fsuid, fdgid, and auxiliary groups.

            let uid = Uid::from_raw(uid_num);
            let gid = Gid::from_raw(gid_num);
            setfsuid(uid);
            setfsgid(gid);

            // Need to use the group permissions of the auxiliary group mihomo
            if let Ok(user_name) = env::var("SUDO_USER") {
                let user_name = CString::new(user_name).unwrap();
                let _ = initgroups(&user_name, gid);
            }
        }
    }
}

pub fn is_run_as_root() -> bool {
    return geteuid().is_root();
}

pub fn restore_fileop_as_root() {
    setfsuid(Uid::from_raw(0));
    setfsgid(Gid::from_raw(0));
}

pub fn run_as_root() {
    let app_path_binding = env::current_exe()
        .expect("Failed to get current executable path");
    let app_path = app_path_binding.to_str()
        .expect("Failed to convert path to string");

    // Skip the param of exe path
    let params: Vec<String> = env::args().skip(1).collect();

    let mut sudo_cmd = vec![app_path];

    sudo_cmd.extend(params.iter().map(|s| s.as_str()));

    // CLASHTUI_EP: clashtui elevate privileges
    env::set_var("CLASHTUI_EP", "true");        // To distinguish when users manually execute `sudo clashtui`
    let _ = process::Command::new("sudo")
        .args(vec!["--preserve-env=CLASHTUI_EP,XDG_CONFIG_HOME,HOME,USER"])
        //.args(vec!["--preserve-env"])
        .args(&sudo_cmd)
        .exec();
}

pub fn run_as_previous_user() {
    if ! is_clashtui_ep() || env::var("SUDO_USER").is_err() {
        return;
    }

    let user_name = env::var("SUDO_USER").unwrap();

    let app_path_binding = env::current_exe()
        .expect("Failed to get current executable path");
    let app_path = app_path_binding.to_str()
        .expect("Failed to convert path to string");

    // Skip the param of exe path
    let params: Vec<String> = env::args().skip(1).collect();

    let mut sudo_cmd = vec![app_path];

    sudo_cmd.extend(params.iter().map(|s| s.as_str()));

    log::info!("run_as_previous_user: {}", user_name);
    let _ = process::Command::new("sudo")
        .args(["-i", "-u", user_name.as_str()])
        .args(&sudo_cmd)
        .exec();
}

// Is clashtui elevate privileges
pub fn is_clashtui_ep() -> bool {
    if let Ok(str) = env::var("CLASHTUI_EP") {
        if str == "true" {
            return true;
        }
    }

    false
}
