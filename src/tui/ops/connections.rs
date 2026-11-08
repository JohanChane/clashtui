//! Connection operations and their events.

use anyhow::Result;

use crate::functions::restful;
use crate::tui::event::{CoreEvent, emit};

pub(in crate::tui) fn close_all() -> Result<()> {
    restful::connection::terminate_all_connections().map_err(|err| anyhow::anyhow!("{err}"))?;
    emit(CoreEvent::ConnectionsClosed);
    Ok(())
}
