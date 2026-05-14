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

pub(crate) mod profile;
pub(crate) mod template;

newtype_tab!(
    /// This can only be [DualTab], because [Template] needs to update [Profile]
    ///
    /// [Template]: template::Template
    /// [Profile]: profile::Profile
    FileTab(DualTab<profile::Profile, template::Template>),
    "File"
);

pub fn agent_init(mut keymap: serde_yml::Mapping) -> anyhow::Result<()> {
    if let Some(val) = keymap.remove("profile") {
        match val {
            serde_yml::Value::Mapping(map) => {
                crate::tui::agent::check_duplicate_keys("file/profile", &map);
                let (keys, descs) = crate::tui::agent::extract_keymap_with_descs(map)?;
                profile::agent_init(keys);
                profile::init_descs(descs);
            }
            serde_yml::Value::Sequence(seq) => {
                let entries: Vec<crate::tui::agent::Entry> =
                    serde_yml::from_value(serde_yml::Value::Sequence(seq))?;
                crate::tui::agent::check_duplicate_keys_list("file/profile", &entries);
                let (keys, descs, chords) =
                    crate::tui::agent::extract_keymap_list(entries)?;
                profile::agent_init(keys);
                profile::init_descs(descs);
                profile::init_chords(chords);
            }
            _ => anyhow::bail!("file/profile is neither Mapping nor Sequence"),
        }
    }
    if let Some(val) = keymap.remove("template") {
        match val {
            serde_yml::Value::Mapping(map) => {
                crate::tui::agent::check_duplicate_keys("file/template", &map);
                let (keys, descs) = crate::tui::agent::extract_keymap_with_descs(map)?;
                template::agent_init(keys);
                template::init_descs(descs);
            }
            serde_yml::Value::Sequence(seq) => {
                let entries: Vec<crate::tui::agent::Entry> =
                    serde_yml::from_value(serde_yml::Value::Sequence(seq))?;
                crate::tui::agent::check_duplicate_keys_list("file/template", &entries);
                let (keys, descs, chords) =
                    crate::tui::agent::extract_keymap_list(entries)?;
                template::agent_init(keys);
                template::init_descs(descs);
                template::init_chords(chords);
            }
            _ => anyhow::bail!("file/template is neither Mapping nor Sequence"),
        }
    }
    Ok(())
}
