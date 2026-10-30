//! Service policy over user-supplied commands; no backend command construction.

use crate::config::{
    self, CONFIG, CoreType,
    scripts::{ResolvedCommand, ScriptConfig},
};
use anyhow::{Context, Result, bail, ensure};
use std::{
    path::Path,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

pub const CORES: [CoreType; 2] = [CoreType::Mihomo, CoreType::Singbox];
static STATUS_LOCKS: [Mutex<()>; 2] = [Mutex::new(()), Mutex::new(())];
static REVISION: AtomicU64 = AtomicU64::new(0);

pub fn revision() -> u64 {
    REVISION.load(Ordering::Acquire)
}

fn with_operation<T>(job: impl FnOnce() -> Result<T>) -> Result<T> {
    let result = crate::tui::with_terminal(job);
    REVISION.fetch_add(1, Ordering::Release);
    result
}

pub fn core_index(core: CoreType) -> usize {
    match core {
        CoreType::Mihomo => 0,
        CoreType::Singbox => 1,
    }
}

pub fn core_id(core: CoreType) -> &'static str {
    match core {
        CoreType::Mihomo => "mihomo",
        CoreType::Singbox => "singbox",
    }
}

#[derive(Clone)]
pub struct TargetContext {
    pub core: CoreType,
    service_name: String,
    core_bin: String,
    core_config_dir: String,
    core_config_path: String,
    controller: String,
    secret: Option<String>,
}

impl TargetContext {
    pub fn snapshot(core: CoreType) -> Self {
        let (paths, service, controller, secret) = match core {
            CoreType::Mihomo => (
                &CONFIG.cfg_file.mihomo.core,
                &CONFIG.cfg_file.mihomo.core_service,
                &CONFIG.external_controller,
                &CONFIG.secret,
            ),
            CoreType::Singbox => (
                &CONFIG.cfg_file.singbox.core,
                &CONFIG.cfg_file.singbox.core_service,
                &CONFIG.singbox_external_controller,
                &CONFIG.singbox_secret,
            ),
        };
        Self {
            core,
            service_name: service.service_name.clone(),
            core_bin: paths.bin_path.clone(),
            core_config_dir: paths.config_dir.clone(),
            core_config_path: paths.config_path.clone(),
            controller: controller.clone(),
            secret: secret.clone(),
        }
    }

    fn value(&self, name: &str, config_root: &Path, port: &mut Option<u16>) -> Result<String> {
        let raw = match name {
            "core_type" => return Ok(core_id(self.core).into()),
            "service_name" => &self.service_name,
            "core_bin" => &self.core_bin,
            "core_config_dir" => &self.core_config_dir,
            "core_config_path" => &self.core_config_path,
            "proxy_port" => {
                if port.is_none() {
                    let response = super::restful::config::fetch_from(
                        &self.controller,
                        self.secret.as_deref(),
                    )
                    .context("Cannot query target core proxy port")?;
                    *port = Some(proxy_port(
                        valid_port(response.get("mixed-port")),
                        valid_port(response.get("port")),
                    )?);
                }
                return Ok(port.unwrap().to_string());
            }
            _ => bail!("Unknown script variable: {name}"),
        };
        ensure!(
            !raw.is_empty(),
            "{}: missing ${{{name}}}",
            core_id(self.core)
        );
        if name == "service_name" {
            return Ok(raw.clone());
        }
        let path = Path::new(raw);
        if name == "core_bin" && !path.is_absolute() && path.components().count() == 1 {
            let search = std::env::var_os("PATH").context("PATH is missing for core_bin lookup")?;
            let mut names = vec![raw.clone()];
            #[cfg(windows)]
            if path.extension().is_none() {
                names.extend(
                    std::env::var("PATHEXT")
                        .unwrap_or_else(|_| ".EXE;.COM;.BAT;.CMD".into())
                        .split(';')
                        .map(|extension| format!("{raw}{extension}")),
                );
            }
            #[cfg(not(windows))]
            let _ = &mut names;
            for directory in std::env::split_paths(&search) {
                for name in &names {
                    let candidate = directory.join(name);
                    if candidate.is_file() {
                        return Ok(std::path::absolute(candidate)?
                            .to_str()
                            .context("Core path is not UTF-8")?
                            .to_owned());
                    }
                }
            }
            bail!("core_bin {raw:?} was not found in PATH");
        }
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            config_root.join(path)
        };
        Ok(std::path::absolute(path)?
            .to_str()
            .context("Core path is not UTF-8")?
            .to_owned())
    }
}

fn valid_port(value: Option<&serde_json::Value>) -> Option<u16> {
    value?.as_u64()?.try_into().ok().filter(|port| *port != 0)
}

fn proxy_port(mixed_port: Option<u16>, port: Option<u16>) -> Result<u16> {
    mixed_port
        .filter(|v| *v != 0)
        .or(port.filter(|v| *v != 0))
        .context("Target core has no valid mixed-port or port")
}

#[derive(Clone, Debug)]
pub struct PlannedAction {
    pub core: CoreType,
    pub id: String,
    pub command: ResolvedCommand,
}

pub fn plan(
    scripts: &ScriptConfig,
    config_root: &Path,
    target: &TargetContext,
    id: &str,
) -> Result<PlannedAction> {
    let action = scripts.find(target.core, id).with_context(|| {
        format!(
            "{} does not support {id} in scripts.json",
            core_id(target.core)
        )
    })?;
    let mut port = None;
    let command = scripts.resolve(config_root, action, |name| {
        target.value(name, config_root, &mut port)
    })?;
    Ok(PlannedAction {
        core: target.core,
        id: id.into(),
        command,
    })
}

fn run(action: &PlannedAction) -> Result<String> {
    println!("{}: {}", core_id(action.core), action.id);
    let outcome = super::process::run_foreground(&action.command)?;
    ensure!(
        outcome.status.success(),
        "{} {} failed: {}",
        core_id(action.core),
        action.id,
        outcome.status
    );
    Ok(format!("{} {}: OK", core_id(action.core), action.id))
}

pub fn execute(scripts: &ScriptConfig, root: &Path, core: CoreType, id: &str) -> Result<String> {
    let action = plan(scripts, root, &TargetContext::snapshot(core), id)?;
    with_operation(|| run(&action))
}

pub fn execute_current(id: &str) -> Result<String> {
    let core = CONFIG.core_type();
    let root = config::config_dir_path();
    let scripts = ScriptConfig::load(&root)?;
    execute(&scripts, &root, core, id)
}

fn stop_plan(
    scripts: &ScriptConfig,
    root: &Path,
    targets: &[TargetContext; 2],
) -> Result<Vec<PlannedAction>> {
    targets
        .iter()
        .map(|target| plan(scripts, root, target, "stop_core"))
        .collect()
}

/// All stops are attempted. Successful steps remain visible on later failure.
fn run_stops(
    stops: &[PlannedAction],
    mut execute: impl FnMut(&PlannedAction) -> Result<String>,
) -> Result<String> {
    let mut failed = false;
    let mut reports = Vec::new();
    for action in stops {
        match execute(action) {
            Ok(report) => reports.push(report),
            Err(error) => {
                failed = true;
                reports.push(format!("{} {}: {error:#}", core_id(action.core), action.id));
            }
        }
    }
    let report = reports.join("\n");
    ensure!(!failed, "{report}");
    Ok(report)
}

pub fn stop_all(scripts: &ScriptConfig, root: &Path) -> Result<String> {
    let targets = CORES.map(TargetContext::snapshot);
    let stops = stop_plan(scripts, root, &targets)?;
    with_operation(|| run_stops(&stops, run))
}

fn run_switch(
    stops: &[PlannedAction],
    start: &PlannedAction,
    mut execute: impl FnMut(&PlannedAction) -> Result<String>,
    save: impl FnOnce(CoreType) -> Result<()>,
) -> Result<String> {
    let stopped = run_stops(stops, &mut execute)?;
    let started = execute(start)
        .with_context(|| format!("{stopped}\nTarget start failed; core selection unchanged"))?;
    save(start.core).with_context(|| format!("{stopped}\n{started}\nTarget was started, but saving selection failed; previous selection retained"))?;
    Ok(format!(
        "{stopped}\n{started}\nCore switched to {}",
        core_id(start.core)
    ))
}

pub fn switch_core(scripts: &ScriptConfig, root: &Path, target: CoreType) -> Result<String> {
    let targets = CORES.map(TargetContext::snapshot);
    // Resolve every required capability before any side effect.
    let stops = stop_plan(scripts, root, &targets)?;
    let start = plan(scripts, root, &targets[core_index(target)], "start_core")?;
    with_operation(|| {
        run_switch(&stops, &start, run, |target| {
            let mut data = CONFIG.data.lock().unwrap();
            let previous = data.core_type;
            data.core_type = target;
            let result = save_selection(&*data, root);
            if let Err(error) = result {
                data.core_type = previous;
                return Err(error);
            }
            Ok(())
        })
    })
}

fn save_selection(data: &config::database::ProfileManager, root: &Path) -> Result<()> {
    use std::io::Write;
    let bytes = serde_yml::to_string(data)?;
    let temporary = root.join(format!(
        ".clashtui.db.{}.{}.tmp",
        std::process::id(),
        fastrand::u64(..)
    ));
    let result = (|| -> Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes.as_bytes())?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, root.join("clashtui.db"))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceState {
    Active,
    Inactive,
    Missing,
    Unknown,
    Unsupported,
    Checking,
}

impl ServiceState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Missing => "missing",
            Self::Unknown => "unknown",
            Self::Unsupported => "unsupported",
            Self::Checking => "...",
        }
    }
}

#[derive(Clone, Debug)]
pub struct StatusReport {
    pub state: ServiceState,
    pub diagnostic: Option<String>,
}

impl StatusReport {
    pub fn unknown(error: impl std::fmt::Display) -> Self {
        Self {
            state: ServiceState::Unknown,
            diagnostic: Some(error.to_string()),
        }
    }
}

fn parse_status(output: &[u8]) -> Result<ServiceState> {
    let output =
        std::str::from_utf8(output).context("status_core stdout must be UTF-8 without BOM")?;
    let line = output
        .strip_suffix("\r\n")
        .or_else(|| output.strip_suffix('\n'))
        .unwrap_or(output);
    match line {
        "active" => Ok(ServiceState::Active),
        "inactive" => Ok(ServiceState::Inactive),
        "missing" => Ok(ServiceState::Missing),
        _ => bail!("status_core must print exactly active, inactive or missing on one line"),
    }
}

pub fn query_status(scripts: &ScriptConfig, root: &Path, core: CoreType) -> StatusReport {
    if scripts.find(core, "status_core").is_none() {
        return StatusReport {
            state: ServiceState::Unsupported,
            diagnostic: None,
        };
    }
    let Ok(_query) = STATUS_LOCKS[core_index(core)].try_lock() else {
        return StatusReport {
            state: ServiceState::Checking,
            diagnostic: None,
        };
    };
    let result = (|| -> Result<StatusReport> {
        let action = plan(scripts, root, &TargetContext::snapshot(core), "status_core")?;
        let output = super::process::run_capture(&action.command)?;
        let diagnostic = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        ensure!(
            output.status.success(),
            "status_core failed ({}): {diagnostic}",
            output.status
        );
        ensure!(
            !output.stdout_overflow && !output.stderr_overflow,
            "status_core exceeded 64 KiB output limit"
        );
        let state = parse_status(&output.stdout)?;
        Ok(StatusReport {
            state,
            diagnostic: (!diagnostic.is_empty()).then_some(diagnostic),
        })
    })();
    match result {
        Ok(report) => report,
        Err(error) => StatusReport::unknown(format!("{error:#}")),
    }
}

pub fn current_status() -> StatusReport {
    let root = config::config_dir_path();
    match ScriptConfig::load(&root) {
        Ok(scripts) => query_status(&scripts, &root, CONFIG.core_type()),
        Err(error) => StatusReport::unknown(format!("{error:#}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, path::PathBuf};

    fn action(core: CoreType, id: &str) -> PlannedAction {
        PlannedAction {
            core,
            id: id.into(),
            command: ResolvedCommand {
                program: "unused".into(),
                args: vec![],
                cwd: PathBuf::from("/"),
            },
        }
    }

    #[test]
    fn status_protocol_is_exact() {
        for value in [b"active".as_slice(), b"active\n", b"active\r\n"] {
            assert_eq!(parse_status(value).unwrap(), ServiceState::Active);
        }
        assert_eq!(parse_status(b"inactive").unwrap(), ServiceState::Inactive);
        assert_eq!(parse_status(b"missing").unwrap(), ServiceState::Missing);
        for value in [
            b"\xef\xbb\xbfactive".as_slice(),
            b" active",
            b"active\n\n",
            b"active\r",
            b"active\ninactive",
            b"error",
            b"\xff",
            b"",
        ] {
            assert!(parse_status(value).is_err(), "{value:?}");
        }
    }

    #[test]
    fn port_zero_falls_back_and_missing_fails() {
        assert_eq!(proxy_port(Some(0), Some(7890)).unwrap(), 7890);
        assert_eq!(proxy_port(Some(7891), Some(7890)).unwrap(), 7891);
        assert!(proxy_port(None, Some(0)).is_err());
        for invalid in [
            serde_json::json!(-1),
            serde_json::json!(65536),
            serde_json::json!("bad"),
        ] {
            assert_eq!(
                proxy_port(valid_port(Some(&invalid)), Some(7890)).unwrap(),
                7890
            );
        }
    }

    #[test]
    fn stops_attempt_all_and_report_partial_success() {
        let stops = CORES.map(|core| action(core, "stop_core"));
        let mut seen = Vec::new();
        let error = run_stops(&stops, |action| {
            seen.push(action.core);
            if action.core == CoreType::Mihomo {
                bail!("denied");
            }
            Ok("singbox stopped".into())
        })
        .unwrap_err()
        .to_string();
        assert_eq!(seen, CORES);
        assert!(error.contains("denied") && error.contains("singbox stopped"));
    }

    #[test]
    fn switch_never_starts_after_failed_stop_or_saves_failed_start() {
        let stops = CORES.map(|core| action(core, "stop_core"));
        let start = action(CoreType::Singbox, "start_core");
        for failed in ["stop_core", "start_core"] {
            let mut seen = Vec::new();
            let saved = Cell::new(false);
            assert!(
                run_switch(
                    &stops,
                    &start,
                    |action| {
                        seen.push(action.id.clone());
                        if action.id == failed {
                            bail!("failed");
                        }
                        Ok("stopped".into())
                    },
                    |_| {
                        saved.set(true);
                        Ok(())
                    }
                )
                .is_err()
            );
            assert!(!saved.get());
            assert_eq!(seen.len(), if failed == "stop_core" { 2 } else { 3 });
        }
    }

    #[test]
    fn switch_saves_after_target_success_and_reports_save_failure() {
        let stops = CORES.map(|core| action(core, "stop_core"));
        let start = action(CoreType::Singbox, "start_core");
        let calls = Cell::new(0);
        let error = run_switch(
            &stops,
            &start,
            |_| {
                calls.set(calls.get() + 1);
                Ok("OK".into())
            },
            |target| {
                assert_eq!(target, CoreType::Singbox);
                assert_eq!(calls.get(), 3);
                bail!("disk full")
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("Target was started"));
    }

    #[test]
    fn target_values_are_lazy_and_preserve_literal_data() {
        let target = TargetContext {
            core: CoreType::Singbox,
            service_name: "服务 $(id) ${core_type}".into(),
            core_bin: String::new(),
            core_config_dir: String::new(),
            core_config_path: String::new(),
            controller: "unreachable".into(),
            secret: None,
        };
        let mut port = None;
        assert_eq!(
            target
                .value("service_name", Path::new("/tmp"), &mut port)
                .unwrap(),
            target.service_name
        );
        assert_eq!(
            target
                .value("core_type", Path::new("/tmp"), &mut port)
                .unwrap(),
            "singbox"
        );
        assert!(
            target
                .value("core_bin", Path::new("/tmp"), &mut port)
                .is_err()
        );
        assert!(port.is_none());
    }

    #[test]
    fn plan_routes_by_target_and_freezes_expanded_values() {
        let scripts = ScriptConfig::parse(r#"{"root":".","actions":[
            {"id":"start_core","label":"M","cores":["mihomo"],"cmd":{"program":"m-tool","args":["${service_name}"]}},
            {"id":"start_core","label":"S","cores":["singbox"],"cmd":{"program":"s-tool","args":["${core_type}","${service_name}"]}}
        ]}"#).unwrap();
        let mut target = TargetContext {
            core: CoreType::Singbox,
            service_name: "original service".into(),
            core_bin: String::new(),
            core_config_dir: String::new(),
            core_config_path: String::new(),
            controller: "unreachable".into(),
            secret: None,
        };
        let root = std::env::temp_dir();
        let planned = plan(&scripts, &root, &target, "start_core").unwrap();
        target.service_name = "changed".into();
        assert_eq!(planned.command.program, "s-tool");
        assert_eq!(planned.command.args, ["singbox", "original service"]);
        assert!(plan(&scripts, &root, &target, "stop_core").is_err());
    }

    #[test]
    fn selection_save_is_atomic_on_serialization_and_rename_failure() {
        let root = std::env::temp_dir().join(format!(
            "clashtui-selection-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        std::fs::create_dir(&root).unwrap();
        let data = config::database::ProfileManager {
            core_type: CoreType::Singbox,
            ..Default::default()
        };
        save_selection(&data, &root).unwrap();
        let loaded: config::database::ProfileManager =
            serde_yml::from_str(&std::fs::read_to_string(root.join("clashtui.db")).unwrap())
                .unwrap();
        assert_eq!(loaded.core_type, CoreType::Singbox);
        std::fs::remove_file(root.join("clashtui.db")).unwrap();
        std::fs::create_dir(root.join("clashtui.db")).unwrap();
        assert!(save_selection(&data, &root).is_err());
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        std::fs::remove_dir_all(root).unwrap();
    }
}
