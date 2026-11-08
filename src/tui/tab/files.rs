use ratatui::{
    text::{Line, Span},
    widgets::ListItem,
};

use super::dev::*;

macro_rules! get_name {
    ($self:expr, $state:expr) => {
        if let Some(idx) = $state.selected() {
            if idx < $self.items.len() {
                $self.items[idx].clone()
            } else {
                return;
            }
        } else {
            return;
        }
    };
}

pub(crate) mod profile;
pub(crate) mod template;

/// Profile and Template. Focus switch stays at the tab, not inside either page.
#[derive(Default)]
pub struct FileTab(crate::tui::page::Split<profile::Profile, template::Template>);

impl crate::tui::tab::TuiTab for FileTab {
    fn title(&self) -> &'static str {
        "File"
    }

    fn input_scope(&self) -> crate::tui::key::Scope {
        match self.0.focus() {
            crate::tui::page::Side::Left => crate::tui::key::Scope::Profile,
            crate::tui::page::Side::Right => crate::tui::key::Scope::Template,
        }
    }

    fn handle_action(
        &mut self,
        action: crate::tui::key::Action,
        commands: &mut crate::tui::commands::Commands,
    ) {
        use crate::tui::key::Action::{Profile, Template};
        match action {
            Profile(profile::Action::Switch) | Template(template::Action::Switch) => {
                self.0.toggle_focus()
            }
            Profile(action) => self.0.left_action(action, commands),
            Template(action) => self.0.right_action(action, commands),
            _ => unreachable!("action routed to the wrong page"),
        }
    }

    fn render(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        self.0.render(f, area);
    }

    fn deliver(&mut self, event: &crate::tui::event::Event) {
        self.0.deliver(event);
    }

    fn command_done(&mut self, event: &crate::tui::commands::CommandEvent) {
        self.0.command_done(event);
    }
    fn sync(&mut self, now: std::time::Instant) {
        self.0.sync(now);
    }
}

/// Match ratatui's clamping when a filter shrinks a list under its cursor.
pub(super) fn selected<I: IntoIterator>(items: I, index: Option<usize>) -> Option<I::Item> {
    items.into_iter().take(index?.saturating_add(1)).last()
}

pub(super) fn path_text(path: &std::path::Path) -> String {
    let text = path.to_string_lossy();
    if cfg!(windows) {
        crate::scripts::variables::sanitize_windows_path(&text)
    } else {
        text.into_owned()
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
        let mut input = Matcher::new(&maps);
        assert_eq!(tab.input_scope(), Scope::Profile);
        for scope in [Scope::Template, Scope::Profile] {
            let current = tab.input_scope();
            let Match::Execute(action) = input.feed(&"l".parse().unwrap(), current, current) else {
                panic!("the pane switch must resolve to an action");
            };
            tab.handle_action(action, &mut crate::tui::commands::Commands::default());
            assert_eq!(tab.input_scope(), scope);
            let result = input.feed(&"x".parse().unwrap(), scope, scope);
            if scope == Scope::Template {
                assert!(matches!(
                    result,
                    Match::Execute(Action::Template(template::Action::Operation(
                        template::Operation::Generate
                    )))
                ));
                assert!(
                    maps.page_bindings(scope)
                        .iter()
                        .any(|b| b.key_text == "x" && b.desc == "Generate profile")
                );
            } else {
                assert!(matches!(
                    result,
                    Match::Execute(Action::Profile(profile::Action::Operation(
                        profile::Operation::Preview
                    )))
                ));
                assert!(
                    maps.page_bindings(scope)
                        .iter()
                        .any(|b| b.key_text == "x" && b.desc == "Preview profile")
                );
            }
        }
    }
}
