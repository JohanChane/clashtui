//! Script definitions and action protocol, independent of the caller's UI.
mod action;
pub mod config;
pub(crate) mod variables;
pub use action::{ActionRequest, ActionResult, ServiceState, StatusReport};
use anyhow::{Result, anyhow};
pub use config::{OutputMode, ScriptConfig};
use std::{path::Path, sync::OnceLock};

static SCRIPT_CONFIG: OnceLock<std::result::Result<ScriptConfig, String>> = OnceLock::new();
pub fn init(root: &Path) -> Result<()> {
    SCRIPT_CONFIG.get_or_init(|| config::load(root).map_err(|error| format!("{error:#}")));
    get().map(|_| ())
}
pub fn get() -> Result<&'static ScriptConfig> {
    SCRIPT_CONFIG
        .get()
        .ok_or_else(|| anyhow!("Scripts have not been initialized"))?
        .as_ref()
        .map_err(|error| anyhow!(error.clone()))
}
