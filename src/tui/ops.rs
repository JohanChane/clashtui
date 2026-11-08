//! Shared operations that modify the core or files. They emit events on success; callers need not emit.
//! Service commands emit ServiceCommandDone on completion, whether they succeed or fail.

use anyhow::{Context, Result, ensure};

use crate::config::database::Profile;
use crate::config::{CONFIG, CoreType};
use crate::functions::file::profile::{self, SelectionOutcome, UpdateResult};
use crate::functions::{command, restful};
use crate::tui::event::{CoreEvent, FileEvent, emit};

pub fn restart_core() -> Result<String> {
    service(command::restart_service())
}
pub fn stop_core() -> Result<String> {
    service(command::stop_service())
}
pub fn stop_all_cores() -> Result<String> {
    service(command::stop_all_services())
}
pub fn start_core(core: CoreType) -> Result<String> {
    service(command::start_core_service(core))
}
#[cfg(windows)]
pub fn install_core(core: CoreType) -> Result<String> {
    service(command::install_core_service(core))
}
#[cfg(windows)]
pub fn uninstall_core(core: CoreType) -> Result<String> {
    service(command::uninstall_core_service(core))
}

/// Stop all cores, save the new core type, start the new core, and return a report for the user.
/// Calls stop_all_cores and start_core, each emitting ServiceCommandDone.
/// The caller emits Switched after the user confirms.
pub fn switch_core(core: CoreType) -> Result<String> {
    let stop = stop_all_cores();
    CONFIG.data.lock().unwrap().core_type = core;
    CONFIG.save().context("save core type")?;
    let start = start_core(core);
    let label = core.to_string();
    Ok(format!(
        "Core switched to {label}\n\nStop all services: {stop}\nStart {label}: {start}",
        stop = brief(&stop),
        start = brief(&start),
    ))
}

pub fn patch_config(payload: String) -> Result<()> {
    restful::config::patch(payload).map_err(|err| anyhow::anyhow!("{err}"))?;
    emit(CoreEvent::ConfigChanged);
    Ok(())
}

pub fn close_all_connections() -> Result<()> {
    restful::connection::terminate_all_connections().map_err(|err| anyhow::anyhow!("{err}"))?;
    emit(CoreEvent::ConnectionsClosed);
    Ok(())
}

/// Save the selection and ask the core to reload its configuration.
pub async fn select_profile(profile: Profile) -> Result<()> {
    profile::select(profile).await?;
    emit(CoreEvent::ConfigChanged);
    emit(FileEvent::ProfilesChanged);
    Ok(())
}

/// Update a profile. If it is the current profile, the underlying operation selects and reloads it.
pub async fn update_profile(profile: Profile, with_proxy: bool) -> Result<UpdateResult> {
    let result = profile::update_profile(profile, with_proxy).await?;
    emit(FileEvent::ProfilesChanged);
    if matches!(result.selection, Some(Ok(SelectionOutcome::Reloaded))) {
        emit(CoreEvent::ConfigChanged);
    }
    Ok(result)
}

/// Emit ServiceCommandDone on completion, even on failure: failed commands may change service state.
/// Convert output starting with "Error" to Err so callers need not check it themselves.
fn service(output: Result<String>) -> Result<String> {
    emit(CoreEvent::ServiceCommandDone);
    let output = output?;
    ensure!(!output.starts_with("Error"), "{output}");
    Ok(output)
}

fn brief(result: &Result<String>) -> String {
    match result {
        Ok(text) => text.trim().to_owned(),
        Err(err) => err.to_string(),
    }
}
