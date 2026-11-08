//! Profile selection and updates with core and file events.

use anyhow::Result;

use crate::config::database::Profile;
use crate::functions::file::profile::{self as file_profile, SelectionOutcome, UpdateResult};
use crate::tui::event::{CoreEvent, FileEvent, emit};

/// Save the selection and ask the core to reload its configuration.
pub(in crate::tui) async fn select(profile: Profile) -> Result<()> {
    file_profile::select(profile).await?;
    emit(CoreEvent::ConfigChanged);
    emit(FileEvent::ProfilesChanged);
    Ok(())
}

/// Update a profile. If it is the current profile, the underlying operation selects and reloads it.
pub(in crate::tui) async fn update(profile: Profile, with_proxy: bool) -> Result<UpdateResult> {
    let result = file_profile::update_profile(profile, with_proxy).await?;
    emit(FileEvent::ProfilesChanged);
    if matches!(result.selection, Some(Ok(SelectionOutcome::Reloaded))) {
        emit(CoreEvent::ConfigChanged);
    }
    Ok(result)
}
