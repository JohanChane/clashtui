use ratatui::{
    text::{Line, Span},
    widgets::ListItem,
};

use super::dev::*;
use crate::tui::TuiWidget;
use load::LoadSlot;
use std::task::{Context, Poll};

/// Record a completed mutation independently of its disposable page callback.
/// App advances the visible Files owner after each action or completion.
macro_rules! sync {
    ($ident: ty) => {{
        $crate::tui::tab::files::operations::mark_changed();
        do_nothing::<$ident>()
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

mod load;
mod operations;
pub(crate) mod profile;
pub(crate) mod template;

pub(in crate::tui) fn request_refresh() {
    operations::mark_changed();
}

/// This can only be [DualTab], because [Template] needs to update [Profile]
///
/// [Template]: template::Template
/// [Profile]: profile::Profile
#[derive(Default)]
pub struct FileTab {
    panes: DualTab<profile::Profile, template::Template>,
    profiles: LoadSlot<(Vec<String>, Vec<String>)>,
    templates: LoadSlot<Vec<String>>,
}
impl TuiWidget for FileTab {
    fn on_enter(&mut self, business: &mut Business) {
        self.panes.on_enter(business);
        self.profiles.enter();
        self.templates.enter();
        self.advance();
    }
    fn on_leave(&mut self, business: &mut Business) {
        self.profiles.leave();
        self.templates.leave();
        self.panes.on_leave(business);
    }
    fn sync_from(&mut self, view: BusinessView<'_>) {
        self.panes.sync_from(view);
    }
    fn render(&mut self, f: &mut Frame, area: Rect, view: BusinessView<'_>) {
        self.panes.render(f, area, view);
    }
    fn advance(&mut self) {
        let revision = operations::data_revision();
        self.profiles
            .advance(revision, || Ok(profile::get_profiles_with_readable_atime()));
        self.templates.advance(revision, || {
            crate::functions::file::template::get_all_templates().map_err(|e| e.to_string())
        });
    }
    fn poll_tasks(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if self.profiles.poll(cx).is_ready()
            || self.templates.poll(cx).is_ready()
            || self.panes.poll_tasks(cx).is_ready()
        {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
    fn finish_task(&mut self, visible: bool) -> bool {
        let mut changed = self.panes.finish_task(visible);
        if let Some(result) = self.profiles.finish() {
            match result {
                Ok(items) => self.panes.content_mut().0.apply_loaded(items),
                Err(error) => Confirm::err(error),
            }
            changed = true;
        }
        if let Some(result) = self.templates.finish() {
            match result {
                Ok(items) => self.panes.content_mut().1.apply_loaded(items),
                Err(error) => Confirm::err(error),
            }
            changed = true;
        }
        // Even a rejected old result releases the real job; chase the latest revision now.
        self.advance();
        changed
    }
    fn has_tasks(&self) -> bool {
        self.profiles.busy() || self.templates.busy() || self.panes.has_tasks()
    }
    fn needs_animation(&self) -> bool {
        self.panes.needs_animation()
    }
}

impl crate::tui::tab::TuiTab for FileTab {
    fn title(&self) -> &'static str {
        "File"
    }

    fn input_scope(&self) -> crate::tui::key::Scope {
        if self.panes.is_focused_on_left() {
            crate::tui::key::Scope::Profile
        } else {
            crate::tui::key::Scope::Template
        }
    }

    fn handle_action(&mut self, action: crate::tui::key::Action, cx: &mut Business) {
        match action {
            crate::tui::key::Action::Profile(action) => self.panes.handle_left_action(action, cx),
            crate::tui::key::Action::Template(action) => self.panes.handle_right_action(action, cx),
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
        // Creating the tab initializes local state without starting file loading.
        let mut tab = FileTab::default();
        let mut cx = Business::default();
        let mut input = Matcher::new(&maps);
        assert_eq!(tab.input_scope(), Scope::Profile);
        for scope in [Scope::Template, Scope::Profile] {
            let current = tab.input_scope();
            let Match::Execute(action) = input.feed(&"l".parse().unwrap(), current, current) else {
                panic!("the pane switch must resolve to an action");
            };
            tab.handle_action(action, &mut cx);
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
