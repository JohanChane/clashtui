//! Embedded defaults and no-clobber extraction into the existing config root.

use super::{CORES, FILE_NAME, ScriptConfig, core_name};
use crate::config::ConfigFile;
use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use std::{
    fs::{self, File, OpenOptions},
    io::{ErrorKind, Write},
    path::{Component, Path, PathBuf},
};

struct Asset {
    path: &'static str,
    bytes: &'static [u8],
    executable: bool,
}

macro_rules! asset {
    ($path:literal, $executable:expr) => {
        Asset {
            path: $path,
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/contrib/scripts/coresrvctrl/",
                $path
            )),
            executable: $executable,
        }
    };
}

const ASSETS: &[Asset] = &[
    asset!("defaults.json", false),
    asset!("systemd/service.sh", true),
    asset!("systemd/system.json", false),
    asset!("systemd/user.json", false),
    asset!("openrc/service.sh", true),
    asset!("openrc/system.json", false),
    asset!("openrc/user.json", false),
    asset!("launchd/service.sh", true),
    asset!("launchd/system.json", false),
    asset!("launchd/user.json", false),
    asset!("windows/service.ps1", false),
    asset!("windows/sysproxy.ps1", false),
    asset!("windows/system.json", false),
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Defaults {
    platforms: Vec<Platform>,
    templates: Vec<Template>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Platform {
    os: String,
    default_controller: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Template {
    os: String,
    controller: String,
    scope: String,
    path: String,
}

/// Extract missing assets and migrate legacy settings only when scripts.json is absent.
/// The caller may report failure without disabling unrelated application features.
pub fn bootstrap(config_root: &Path, config: &ConfigFile) -> Result<()> {
    bootstrap_for_os(config_root, config, std::env::consts::OS)
}

fn bootstrap_for_os(config_root: &Path, config: &ConfigFile, os: &str) -> Result<()> {
    let config_root = std::path::absolute(config_root).context("Resolve CONFIG_ROOT")?;
    reject_symlinks(&config_root)?;
    ensure!(
        config_root.is_dir(),
        "{} is not a directory",
        config_root.display()
    );
    let destination = config_root.join(FILE_NAME);
    let generated = if regular_file_exists(&destination)? {
        // Invalid user JSON is an error, never a reason to replace it using old fields.
        ScriptConfig::load(&config_root)?;
        None
    } else {
        // Validate every core before publishing any new command configuration.
        Some(migrate(config, os)?)
    };

    let scripts_dir = config_root.join("scripts");
    ensure_directory(&scripts_dir)?;
    let resource_root = scripts_dir.join("coresrvctrl");
    ensure_directory(&resource_root)?;
    for asset in ASSETS {
        let relative = Path::new(asset.path);
        let mut parent = resource_root.clone();
        if let Some(path) = relative.parent() {
            for component in path.components() {
                ensure!(
                    matches!(component, Component::Normal(_)),
                    "Invalid embedded path: {}",
                    asset.path
                );
                parent.push(component.as_os_str());
                ensure_directory(&parent)?;
            }
        }
        let target = resource_root.join(relative);
        if !publish_if_missing(&target, asset.bytes, asset.executable)? {
            // Preserve the user's file; make the current default available for manual adoption.
            let existing = fs::read(&target)
                .with_context(|| format!("Failed to read {}", target.display()))?;
            if existing != asset.bytes {
                let mut sample = target.as_os_str().to_owned();
                sample.push(".new");
                publish_if_missing(Path::new(&sample), asset.bytes, asset.executable)?;
            }
        }
    }

    if let Some(generated) = generated {
        let mut bytes = serde_json::to_vec_pretty(&generated).context("Serialize scripts.json")?;
        bytes.push(b'\n');
        publish_if_missing(&destination, &bytes, false)?;
    }
    // Another process may have created the destination while resources were extracted.
    ScriptConfig::load(&config_root)?;
    Ok(())
}

fn embedded(path: &str) -> Result<&'static [u8]> {
    ASSETS
        .iter()
        .find(|asset| asset.path == path)
        .map(|asset| asset.bytes)
        .with_context(|| format!("Missing embedded script resource: {path}"))
}

fn migrate(config: &ConfigFile, os: &str) -> Result<ScriptConfig> {
    let defaults: Defaults = serde_json::from_slice(embedded("defaults.json")?)
        .context("Invalid embedded defaults.json")?;
    let platform = defaults
        .platforms
        .iter()
        .find(|platform| platform.os == os)
        .with_context(|| format!("No default script configuration for OS {os:?}"))?;
    let mut merged: Option<ScriptConfig> = None;
    for core in CORES {
        let legacy = match core {
            super::CoreType::Mihomo => &config.mihomo.core_service,
            super::CoreType::Singbox => &config.singbox.core_service,
        };
        let controller = legacy
            .service_controller
            .as_deref()
            .unwrap_or(&platform.default_controller);
        let scope = if legacy.is_user { "user" } else { "system" };
        let candidates = defaults
            .templates
            .iter()
            .filter(|template| {
                template.os == os && template.controller == controller && template.scope == scope
            })
            .collect::<Vec<_>>();
        ensure!(
            candidates.len() == 1,
            "No unique script template for {}: OS={os}, controller={controller:?}, scope={scope}",
            core_name(core)
        );
        let template = candidates[0];
        let text = std::str::from_utf8(embedded(&template.path)?)
            .with_context(|| format!("Template {} is not UTF-8", template.path))?;
        let source = ScriptConfig::parse(text)
            .with_context(|| format!("Invalid embedded template {}", template.path))?;
        let result = merged.get_or_insert_with(|| ScriptConfig {
            root: source.root.clone(),
            actions: vec![],
        });
        ensure!(
            result.root == source.root,
            "Embedded templates use different script roots"
        );
        for action in source.actions_for(core) {
            if let Some(existing) = result
                .actions
                .iter_mut()
                .find(|item| item.id == action.id && item.cmd == action.cmd)
            {
                existing
                    .cores
                    .as_mut()
                    .expect("migration scopes are explicit")
                    .push(core);
            } else {
                let mut action = action.clone();
                action.cores = Some(vec![core]);
                result.actions.push(action);
            }
        }
    }
    let mut result = merged.context("No core script templates selected")?;
    for action in &mut result.actions {
        if action
            .cores
            .as_ref()
            .is_some_and(|cores| cores.len() == CORES.len())
        {
            action.cores = None;
        }
    }
    result
        .validate()
        .context("Migrated scripts.json is invalid")?;
    Ok(result)
}

/// Check existing ancestors as well as the target; dangling symlinks count too.
pub(super) fn reject_symlinks(path: &Path) -> Result<()> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) => ensure!(
                !metadata.file_type().is_symlink(),
                "Refusing symbolic link: {}",
                current.display()
            ),
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("Failed to inspect {}", current.display()));
            }
        }
    }
    Ok(())
}

fn regular_file_exists(path: &Path) -> Result<bool> {
    reject_symlinks(path)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            ensure!(
                metadata.is_file(),
                "Expected a regular file: {}",
                path.display()
            );
            Ok(true)
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("Failed to inspect {}", path.display())),
    }
}

fn ensure_directory(path: &Path) -> Result<()> {
    reject_symlinks(path)?;
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    match builder.create(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            reject_symlinks(path)?;
            ensure!(path.is_dir(), "Expected a directory: {}", path.display());
            Ok(())
        }
        Err(error) => Err(error).with_context(|| format!("Failed to create {}", path.display())),
    }
}

struct TemporaryFile(PathBuf);

impl Drop for TemporaryFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// Publish a fully written file without replacing a destination created by
/// another clashtui or the user concurrently.
fn publish_if_missing(path: &Path, bytes: &[u8], executable: bool) -> Result<bool> {
    if regular_file_exists(path)? {
        return Ok(false);
    }
    let parent = path
        .parent()
        .context("Script resource has no parent directory")?;
    let (temporary, mut file) = create_temporary(parent, executable)?;
    file.write_all(bytes)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("Failed to sync {}", path.display()))?;
    drop(file);
    reject_symlinks(path)?;
    match publish_temporary(&temporary.0, path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            ensure!(
                regular_file_exists(path)?,
                "Destination disappeared: {}",
                path.display()
            );
            Ok(false)
        }
        Err(error) => {
            Err(error).with_context(|| format!("Failed to atomically create {}", path.display()))
        }
    }
}

#[cfg(not(windows))]
fn publish_temporary(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::hard_link(source, destination)
}

#[cfg(windows)]
fn publish_temporary(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    // Unlike a hard link, a no-replace move also supports portable FAT volumes.
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(source: *const u16, destination: *const u16, flags: u32) -> i32;
    }
    let source = source
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    if source[..source.len() - 1].contains(&0) || destination[..destination.len() - 1].contains(&0)
    {
        return Err(std::io::Error::new(
            ErrorKind::InvalidInput,
            "NUL in resource path",
        ));
    }
    // No MOVEFILE_REPLACE_EXISTING: an existing user file must win the race.
    let result = unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), 0) };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn create_temporary(parent: &Path, executable: bool) -> Result<(TemporaryFile, File)> {
    reject_symlinks(parent)?;
    for _ in 0..16 {
        let path = parent.join(format!(
            ".clashtui-{}-{:016x}.tmp",
            std::process::id(),
            fastrand::u64(..)
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(if executable { 0o700 } else { 0o600 });
        }
        #[cfg(not(unix))]
        let _ = executable;
        match options.open(&path) {
            Ok(file) => return Ok((TemporaryFile(path), file)),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("Failed to create temporary file in {}", parent.display())
                });
            }
        }
    }
    bail!("Could not allocate temporary file in {}", parent.display())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CoreType;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "clashtui-scripts-{}-{:016x}",
                std::process::id(),
                fastrand::u64(..)
            ));
            fs::create_dir(&path).unwrap();
            Self(path.canonicalize().unwrap())
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn offline_bootstrap_extracts_assets_and_one_config() {
        let dir = TempDir::new();
        fs::write(
            dir.0.join("config.yaml"),
            "user configuration stays unchanged",
        )
        .unwrap();
        bootstrap_for_os(&dir.0, &ConfigFile::default(), "linux").unwrap();
        let config = ScriptConfig::load(&dir.0).unwrap();
        assert_eq!(config.actions.len(), 6);
        assert!(config.actions.iter().all(|action| action.cores.is_none()));
        for asset in ASSETS {
            assert_eq!(
                fs::read(dir.0.join("scripts/coresrvctrl").join(asset.path)).unwrap(),
                asset.bytes
            );
        }
        assert_eq!(
            fs::read_to_string(dir.0.join("config.yaml")).unwrap(),
            "user configuration stays unchanged"
        );
        assert!(!dir.0.join("scripts-mihomo.json").exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(dir.0.join("scripts/coresrvctrl/systemd/service.sh"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o700);
        }
    }

    #[test]
    fn migration_preserves_different_backends_and_privileges() {
        let mut legacy = ConfigFile::default();
        legacy.singbox.core_service.is_user = true;
        legacy.singbox.core_service.service_controller = Some("openrc".into());
        let config = migrate(&legacy, "linux").unwrap();
        let mihomo = config.find(CoreType::Mihomo, "restart_core").unwrap();
        let singbox = config.find(CoreType::Singbox, "restart_core").unwrap();
        assert_eq!(mihomo.cores, Some(vec![CoreType::Mihomo]));
        assert_eq!(singbox.cores, Some(vec![CoreType::Singbox]));
        assert_eq!(mihomo.cmd.program, "sudo");
        assert_eq!(singbox.cmd.program, "/bin/sh");
        assert!(
            singbox
                .cmd
                .args
                .iter()
                .any(|arg| arg.contains("openrc/service.sh"))
        );
        assert!(singbox.cmd.args.iter().any(|arg| arg == "user"));
    }

    #[test]
    fn every_embedded_template_validates_and_uses_actual_start() {
        for asset in ASSETS
            .iter()
            .filter(|asset| asset.path.ends_with(".json") && asset.path != "defaults.json")
        {
            let template = ScriptConfig::parse(std::str::from_utf8(asset.bytes).unwrap()).unwrap();
            let start = template.find(CoreType::Mihomo, "start_core").unwrap();
            assert!(
                start.cmd.args.iter().any(|arg| arg == "start"),
                "{}",
                asset.path
            );
            assert!(
                !start.cmd.args.iter().any(|arg| arg == "restart"),
                "{}",
                asset.path
            );
            for id in [
                "stop_core",
                "restart_core",
                "status_core",
                "enable_autostart_core",
                "disable_autostart_core",
            ] {
                assert!(
                    template.find(CoreType::Singbox, id).is_some(),
                    "{} missing {id}",
                    asset.path
                );
            }
        }
        for os in ["linux", "macos", "windows"] {
            migrate(&ConfigFile::default(), os).unwrap();
        }
    }

    #[test]
    fn unknown_legacy_backend_never_publishes_partial_configuration() {
        let dir = TempDir::new();
        let mut legacy = ConfigFile::default();
        legacy.singbox.core_service.service_controller = Some("unknown-backend".into());
        let error = format!(
            "{:#}",
            bootstrap_for_os(&dir.0, &legacy, "linux").unwrap_err()
        );
        assert!(error.contains("singbox"), "{error}");
        assert!(error.contains("unknown-backend"), "{error}");
        assert!(!dir.0.join(FILE_NAME).exists());
    }

    #[test]
    fn existing_configuration_wins_over_invalid_legacy_values() {
        let dir = TempDir::new();
        let json = b"{\"root\":\"custom\",\"actions\":[]}";
        fs::write(dir.0.join(FILE_NAME), json).unwrap();
        let mut legacy = ConfigFile::default();
        legacy.mihomo.core_service.service_controller = Some("custom-backend".into());
        bootstrap_for_os(&dir.0, &legacy, "unknown-os").unwrap();
        assert_eq!(fs::read(dir.0.join(FILE_NAME)).unwrap(), json);
    }

    #[test]
    fn invalid_existing_json_is_preserved_without_fallback() {
        let dir = TempDir::new();
        fs::write(dir.0.join(FILE_NAME), b"{bad JSON").unwrap();
        assert!(bootstrap_for_os(&dir.0, &ConfigFile::default(), "linux").is_err());
        assert_eq!(fs::read(dir.0.join(FILE_NAME)).unwrap(), b"{bad JSON");
    }

    #[test]
    fn modified_files_and_existing_upgrade_samples_are_preserved() {
        let dir = TempDir::new();
        bootstrap_for_os(&dir.0, &ConfigFile::default(), "linux").unwrap();
        let script = dir.0.join("scripts/coresrvctrl/systemd/service.sh");
        let sample = dir.0.join("scripts/coresrvctrl/systemd/service.sh.new");
        fs::write(&script, "my script").unwrap();
        fs::write(&sample, "my pending update").unwrap();
        let missing = dir.0.join("scripts/coresrvctrl/openrc/service.sh");
        fs::remove_file(&missing).unwrap();
        bootstrap_for_os(&dir.0, &ConfigFile::default(), "linux").unwrap();
        assert_eq!(fs::read_to_string(script).unwrap(), "my script");
        assert_eq!(fs::read_to_string(sample).unwrap(), "my pending update");
        assert_eq!(
            fs::read(missing).unwrap(),
            embedded("openrc/service.sh").unwrap()
        );
    }

    #[test]
    fn concurrent_bootstraps_do_not_overwrite_or_leave_partial_files() {
        let dir = TempDir::new();
        std::thread::scope(|scope| {
            let mut threads = Vec::new();
            for _ in 0..4 {
                threads.push(
                    scope.spawn(|| bootstrap_for_os(&dir.0, &ConfigFile::default(), "linux")),
                );
            }
            for thread in threads {
                thread.join().unwrap().unwrap();
            }
        });
        ScriptConfig::load(&dir.0).unwrap();
        for asset in ASSETS {
            let path = dir.0.join("scripts/coresrvctrl").join(asset.path);
            assert_eq!(fs::read(path).unwrap(), asset.bytes);
        }
        for entry in fs::read_dir(&dir.0).unwrap() {
            assert!(
                !entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".tmp")
            );
        }
    }

    #[test]
    fn write_conflict_does_not_publish_configuration_or_destroy_user_file() {
        let dir = TempDir::new();
        fs::write(dir.0.join("scripts"), "user file").unwrap();
        let error = format!(
            "{:#}",
            bootstrap_for_os(&dir.0, &ConfigFile::default(), "linux").unwrap_err()
        );
        assert!(error.contains("scripts"), "{error}");
        assert_eq!(
            fs::read_to_string(dir.0.join("scripts")).unwrap(),
            "user file"
        );
        assert!(!dir.0.join(FILE_NAME).exists());
    }

    #[cfg(unix)]
    #[test]
    fn read_only_directory_reports_write_failure_without_partial_config() {
        use std::os::unix::fs::PermissionsExt;
        if unsafe { libc::geteuid() } == 0 {
            return; // Root bypasses the directory permissions exercised by this test.
        }
        let dir = TempDir::new();
        fs::set_permissions(&dir.0, fs::Permissions::from_mode(0o500)).unwrap();
        let result = bootstrap_for_os(&dir.0, &ConfigFile::default(), "linux");
        fs::set_permissions(&dir.0, fs::Permissions::from_mode(0o700)).unwrap();
        let error = format!("{:#}", result.unwrap_err());
        assert!(error.contains("scripts"), "{error}");
        assert!(!dir.0.join(FILE_NAME).exists());
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 0);
    }

    #[test]
    fn publication_never_replaces_an_existing_file() {
        let dir = TempDir::new();
        let path = dir.0.join("file");
        assert!(publish_if_missing(&path, b"first", false).unwrap());
        assert!(!publish_if_missing(&path, b"second", false).unwrap());
        assert_eq!(fs::read(path).unwrap(), b"first");
    }

    #[cfg(unix)]
    #[test]
    fn target_and_ancestor_symlinks_are_rejected_even_when_dangling() {
        use std::os::unix::fs::symlink;
        let dir = TempDir::new();
        let outside = TempDir::new();
        symlink(outside.0.join("not-created.json"), dir.0.join(FILE_NAME)).unwrap();
        assert!(bootstrap_for_os(&dir.0, &ConfigFile::default(), "linux").is_err());
        assert!(!outside.0.join("not-created.json").exists());
        fs::remove_file(dir.0.join(FILE_NAME)).unwrap();
        symlink(&outside.0, dir.0.join("scripts")).unwrap();
        assert!(bootstrap_for_os(&dir.0, &ConfigFile::default(), "linux").is_err());
        assert_eq!(fs::read_dir(&outside.0).unwrap().count(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_configuration_root_is_not_followed() {
        use std::os::unix::fs::symlink;
        let dir = TempDir::new();
        let outside = TempDir::new();
        let alias = dir.0.join("config-root");
        symlink(&outside.0, &alias).unwrap();
        let error = format!(
            "{:#}",
            bootstrap_for_os(&alias, &ConfigFile::default(), "linux").unwrap_err()
        );
        assert!(error.contains("config-root"), "{error}");
        assert_eq!(fs::read_dir(&outside.0).unwrap().count(), 0);
    }
}
