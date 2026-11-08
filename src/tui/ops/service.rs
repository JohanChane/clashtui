//! Core service lifecycle operations and their completion events.

use anyhow::{Context, Result, ensure};

use crate::config::{CONFIG, CoreType};
use crate::functions::command;
use crate::tui::event::{CoreEvent, emit};

pub(in crate::tui) fn restart() -> Result<String> {
    service(command::restart_service())
}
pub(in crate::tui) fn stop() -> Result<String> {
    service(command::stop_service())
}
pub(in crate::tui) fn stop_all() -> Result<String> {
    service(command::stop_all_services())
}
fn start(core: CoreType) -> Result<String> {
    service(command::start_core_service(core))
}
#[cfg(windows)]
pub(in crate::tui) fn install(core: CoreType) -> Result<String> {
    service(command::install_core_service(core))
}
#[cfg(windows)]
pub(in crate::tui) fn uninstall(core: CoreType) -> Result<String> {
    service(command::uninstall_core_service(core))
}

/// Stop all cores, save the new core type, start the new core, and return a report for the user.
/// Calls stop_all and start, each emitting ServiceCommandDone.
/// The caller emits Switched after the user confirms.
pub(in crate::tui) fn switch_core(core: CoreType) -> Result<String> {
    let stop = stop_all();
    CONFIG.data.lock().unwrap().core_type = core;
    CONFIG.save().context("save core type")?;
    let start = start(core);
    let label = core.to_string();
    Ok(format!(
        "Core switched to {label}\n\nStop all services: {stop}\nStart {label}: {start}",
        stop = brief(&stop),
        start = brief(&start),
    ))
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
