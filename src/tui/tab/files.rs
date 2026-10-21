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

/// This can only be [DualTab], because [Template] needs to update [Profile]
///
/// [Template]: template::Template
/// [Profile]: profile::Profile
#[derive(Default)]
pub struct FileTab(DualTab<profile::Profile, template::Template>);

crate::new_type_impl_tuiwidget!(FileTab);

impl crate::tui::tab::TuiTab for FileTab {
    fn title(&self) -> &'static str {
        "File"
    }

    fn key_description(&self) -> crate::tui::key::KeyDesc {
        if self.0.is_foucus_on_left() {
            profile::km::get_docs()
        } else {
            template::km::get_docs()
        }
    }

    fn pending_keys(&self) -> Option<&'static crate::tui::key::WhichHints> {
        if self.0.is_foucus_on_left() {
            profile::km::pending_keys()
        } else {
            template::km::pending_keys()
        }
    }

    fn cancel_pending(&self) {
        profile::km::cancel_pending();
        template::km::cancel_pending();
    }

    fn has_binding(&self, key: &crate::tui::Key) -> bool {
        if self.0.is_foucus_on_left() {
            profile::km::has_binding(key)
        } else {
            template::km::has_binding(key)
        }
    }

    fn starts_chord(&self, key: &crate::tui::Key) -> bool {
        if self.0.is_foucus_on_left() {
            profile::km::starts_chord(key)
        } else {
            template::km::starts_chord(key)
        }
    }
}
