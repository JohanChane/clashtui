//! Core runtime configuration changes and their events.

use anyhow::Result;

use crate::functions::restful;
use crate::tui::event::{CoreEvent, emit};

pub(in crate::tui) fn patch(payload: String) -> Result<()> {
    restful::config::patch(payload).map_err(|err| anyhow::anyhow!("{err}"))?;
    emit(CoreEvent::ConfigChanged);
    Ok(())
}
