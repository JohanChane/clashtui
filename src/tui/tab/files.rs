use ratatui::{
    text::{Line, Span},
    widgets::ListItem,
};

use super::dev::*;

/// The Only reason why I use two functions to `sync` is that
/// I except modifying Self (what we do in `wrapper`) is
/// fast and infallable
///
/// Tasks should be done in async{} and left only values that
/// apply to Self
macro_rules! sync {
    ($ident: ty) => {{
        let (name, atime) = super::profile::get_profiles_with_readable_atime();
        wrapper(|(content, _): &mut $ident| super::profile::sync_helper(content, name, atime))
    }};
}

macro_rules! get_name {
    ($self:expr, $state:expr) => {
        if let Some(idx) = $state.selected() {
            if idx < $self.items.len() {
                $self.items[idx].clone()
            } else {
                return false;
            }
        } else {
            return false;
        }
    };
}

mod profile;
mod template;

newtype_tab!(
    /// This can only be [DualTab], because [Template] needs to update [Profile]
    ///
    /// [Template]: template::Template
    /// [Profile]: profile::Profile
    FileTab(DualTab<profile::Profile, template::Template>),
    "File"
);

pub fn agent_init(mut keymap: serde_yml::Mapping) -> anyhow::Result<()> {
    if let Some(map) = keymap.remove("profile") {
        let keys = serde_yml::from_value(map)?;
        profile::agent_init(keys);
    }
    if let Some(map) = keymap.remove("template") {
        let keys = serde_yml::from_value(map)?;
        template::agent_init(keys);
    }
    Ok(())
}
