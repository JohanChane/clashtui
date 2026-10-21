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

    fn input_scope(&self) -> crate::tui::key::Scope {
        if self.0.is_foucus_on_left() {
            crate::tui::key::Scope::Profile
        } else {
            crate::tui::key::Scope::Template
        }
    }

    fn handle_action(&mut self, action: crate::tui::key::Action) {
        match action {
            crate::tui::key::Action::Profile(action) => self.0.handle_left_action(action),
            crate::tui::key::Action::Template(action) => self.0.handle_right_action(action),
            _ => unreachable!("action routed to the wrong page"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::{
        key::{Action, Keymaps, Match, Matcher, Scope},
        tab::TuiTab,
    };

    #[tokio::test(flavor = "current_thread")]
    async fn switching_focus_selects_the_other_panes_bindings_and_help() {
        let maps = Keymaps::from_config(serde_yml::from_str(
            "file:\n  profile: [{on: x, action: Preview, desc: Preview profile}]\n  template: [{on: x, action: Generate, desc: Generate profile}]",
        ).unwrap(), "mihomo").unwrap();
        // No await: background file loading is cancelled when this tab is dropped.
        let mut tab = FileTab::default();
        let mut input = Matcher::default();
        assert_eq!(tab.input_scope(), Scope::Profile);
        for scope in [Scope::Template, Scope::Profile] {
            let current = tab.input_scope();
            let Match::Execute(action) =
                input.feed(&"l".parse().unwrap(), current, &maps.get(current).map)
            else {
                panic!("the pane switch must resolve to an action");
            };
            tab.handle_action(action);
            assert_eq!(tab.input_scope(), scope);
            let bindings = maps.get(scope);
            let result = input.feed(&"x".parse().unwrap(), scope, &bindings.map);
            if scope == Scope::Template {
                assert!(matches!(
                    result,
                    Match::Execute(Action::Template(template::Key::Action(
                        template::Action::Generate
                    )))
                ));
                assert!(
                    bindings
                        .page_docs()
                        .contains(&("x".into(), "Generate profile".into()))
                );
            } else {
                assert!(matches!(
                    result,
                    Match::Execute(Action::Profile(profile::Key::Action(
                        profile::Action::Preview
                    )))
                ));
                assert!(
                    bindings
                        .page_docs()
                        .contains(&("x".into(), "Preview profile".into()))
                );
            }
        }
    }
}
