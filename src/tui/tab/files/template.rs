use super::*;
use crate::functions::file::template::*;
use ratatui::style::Style;
use std::cell::Cell;

key_map!(
    Template,
    Action,
    DefaultBindings::new()
        .with_keys([
            (KeyCode::Left, Action::Switch),
            (KeyCode::Right, Action::Switch),
            (KeyCode::Char('h'), Action::Switch),
            (KeyCode::Char('l'), Action::Switch),
            (KeyCode::Down, Action::MoveDown),
            (KeyCode::Up, Action::MoveUp),
            (KeyCode::Char('j'), Action::MoveDown),
            (KeyCode::Char('k'), Action::MoveUp),
            (KeyCode::Char('D'), Action::Operation(Operation::Delete)),
            (KeyCode::Char('e'), Action::Operation(Operation::Edit)),
            (
                KeyCode::Char('E'),
                Action::Operation(Operation::EditProviders)
            ),
            (KeyCode::Char('p'), Action::Operation(Operation::Preview)),
            (KeyCode::Enter, Action::Operation(Operation::Generate)),
            (KeyCode::Char('f'), Action::Operation(Operation::FzfFind)),
            (KeyCode::Char('G'), Action::GoEnd),
            (KeyCode::Char('/'), Action::Operation(Operation::Search)),
        ])
        .with_chords(KeyCode::Char('g'), [(KeyCode::Char('g'), Action::GoTop)])
        .with_chords(KeyCode::Char('y'), [(KeyCode::Char('p'), Action::CopyPath)])
);

#[derive_aliases::derive(..Action, Debug)]
pub(in crate::tui) enum Action {
    CopyPath,
    Switch,
    MoveUp,
    MoveDown,
    Select,
    GoTop,
    GoEnd,

    // Expose file operations as plain action names in user configuration.
    #[serde(untagged)]
    Operation(Operation),
}
impl Document for Action {
    fn get_doc(&self) -> &'static str {
        use crate::tui::key::consts::*;
        match self {
            Self::CopyPath => "Copy template path",
            Self::Switch => "Switch panel",
            Self::MoveUp => MOVE_UP,
            Self::MoveDown => MOVE_DOWN,
            Self::Select => "Select",
            Self::GoTop => GO_TOP,
            Self::GoEnd => GO_BOTTOM,
            Self::Operation(Operation::Generate) => "Generate",
            Self::Operation(Operation::Delete) => "Delete",
            Self::Operation(Operation::Edit) => "Edit",
            Self::Operation(Operation::EditProviders) => "Edit proxy providers",
            Self::Operation(Operation::Preview) => "Preview",
            Self::Operation(Operation::Search) => FILTER,
            Self::Operation(Operation::FzfFind) => "Find template",
        }
    }
}

#[derive_aliases::derive(..Action, Debug)]
pub(in crate::tui) enum Operation {
    Generate,
    Delete,
    Edit,
    EditProviders,
    Preview,
    Search,
    FzfFind,
}

#[derive(Default)]
pub(super) struct Template {
    items: Vec<String>,
    filter: Option<String>,
    jump_target: Cell<Option<String>>,
}

impl Page for Template {
    type Action = Action;
    type State = ListState;
    const TITLE: &str = "Template";

    fn on_event(&mut self, event: &Event, cx: &mut Cx<Self>) {
        match event {
            Event::Init | Event::Shown => self.reload(cx),
            _ => {}
        }
    }

    fn on_action(&mut self, action: Self::Action, cx: &mut Cx<Self>, state: &mut Self::State) {
        let commands = cx.commands();
        match action {
            Action::CopyPath | Action::Operation(Operation::Edit | Operation::EditProviders) => {
                use crate::scripts::ActionRequest;
                let result = (|| -> anyhow::Result<()> {
                    let core = crate::config::CONFIG.core_type();
                    let path = if matches!(action, Action::Operation(Operation::EditProviders)) {
                        crate::config::core_data_dir(core).join("template_proxy_providers.yaml")
                    } else {
                        let name = super::selected(
                            self.items.iter().filter(|name| {
                                self.filter.as_deref().is_none_or(|pat| name.contains(pat))
                            }),
                            state.selected(),
                        )
                        .ok_or_else(|| anyhow::anyhow!("Nothing selected"))?;
                        crate::functions::file::TEMPLATE_PATH.join(name)
                    };
                    commands.submit(
                        core,
                        if matches!(action, Action::CopyPath) {
                            ActionRequest::Copy(super::path_text(&path))
                        } else {
                            ActionRequest::EditFile(path)
                        },
                    )
                })();
                if let Err(error) = result {
                    Confirm::err(format!("{error:#}"));
                }
            }
            Action::Switch => {}
            Action::MoveDown => state.select_next(),
            Action::MoveUp => state.select_previous(),
            Action::GoTop => state.select_first(),
            Action::GoEnd => state.select_last(),

            Action::Select => todo!(),

            Action::Operation(action) => {
                log::debug!("Template::Operation: {action:?}");
                match action {
                    Operation::FzfFind => {
                        let items = self.items.clone();
                        actions::fzf_find(items).spawn_at(cx);
                    }
                    _ => {
                        let name = get_name!(self, state);
                        log::debug!("Template::Operation name={name}");
                        action.act(name).spawn_at(cx);
                    }
                }
            }
        }
    }

    fn render(&self, f: &mut Frame, area: Rect, state: &mut Self::State, is_focused: bool) {
        if let Some(name) = self.jump_target.take()
            && let Some(index) = self.items.iter().position(|item| item == &name)
        {
            state.select(Some(index));
        }

        // Clamp cursor to valid range
        if let Some(idx) = state.selected() {
            if self.items.is_empty() {
                state.select(None);
            } else if idx >= self.items.len() {
                state.select(Some(self.items.len().saturating_sub(1)));
            }
        } else if !self.items.is_empty() {
            state.select(Some(0));
        }

        let theme = Theme::get();
        let section = theme.section("file");
        let unfocused_border = section.border.fg(Color::Rgb(100, 100, 100));
        let unfocused_highlight = Style::new();

        let block = Block::bordered()
            .border_style(if is_focused {
                section.border
            } else {
                unfocused_border
            })
            .title(Self::TITLE);

        let block = if let Some(filter) = self.filter.as_ref() {
            block.title_bottom(Line::raw(format!(" {filter} ")).right_aligned().reversed())
        } else {
            block
        };

        let iter = self
            .items
            .iter()
            // filter content now
            .filter_map(|value| {
                self.filter
                    .as_deref()
                    .is_none_or(|pat| value.contains(pat))
                    .then_some(value.as_str())
            });
        let widget = List::from_iter(iter)
            .block(block)
            .highlight_style(if is_focused {
                section.highlight
            } else {
                unfocused_highlight
            });
        f.render_stateful_widget(widget, area, state);
    }
}

impl Template {
    fn reload(&mut self, cx: &mut Cx<Self>) {
        async {
            let templates = tri!(get_all_templates());
            wrapper(move |content: &mut Self| content.items = templates)
        }
        .spawn_at(cx);
    }
}

mod actions {
    use super::*;

    impl Operation {
        pub(super) async fn act(self, name: String) -> CB {
            match self {
                Self::Generate => generate(name).await,
                Self::Delete => delete(name).await,
                Self::Edit => unreachable!("Edit is submitted through Commands"),
                Self::EditProviders => unreachable!("Edit is submitted through Commands"),
                Self::Preview => preview(name).await,
                Self::Search => search().await,
                Self::FzfFind => unreachable!("FzfFind handled directly"),
            }
        }
    }

    type CB = Box<dyn for<'a> FnOnce(&'a mut C) + Send + 'static>;
    type C = Template;

    async fn generate(name: String) -> CB {
        let profile_name = format!("{name}.tpl");
        let is_singbox = crate::config::CONFIG.core_type() == crate::config::CoreType::Singbox;
        if is_singbox {
            tri!(apply_template_singbox(&name, &profile_name, false, false).await);
        } else {
            tri!(apply_template(&name, &profile_name));
        }
        emit(FileEvent::ProfilesChanged);
        do_nothing()
    }

    async fn delete(name: String) -> CB {
        let rx = Confirm::title("Delete template?".to_string())
            .with_prompt(format!("Delete {name}?\nEnter to confirm, Esc to cancel"))
            .build_and_send();
        if rx.await.is_err() {
            return do_nothing();
        }

        let path = crate::functions::file::TEMPLATE_PATH.join(&name);
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                Confirm::err(e);
                return do_nothing();
            }
        }

        let templates = tri!(get_all_templates());
        wrapper(move |content: &mut C| {
            content.items = templates;
        })
    }

    async fn preview(_name: String) -> CB {
        todo!()
    }

    async fn search() -> CB {
        let filter = tri!(
            Input::new()
                .with_title("Filter".to_owned())
                .build_and_send()
                .await,
            or_cancel
        );

        wrapper(|content: &mut C| {
            content.filter = (!filter.is_empty()).then_some(filter);
        })
    }

    pub(super) async fn fzf_find(items: Vec<String>) -> CB {
        let selected = FzfFinder::new(items.clone())
            .with_title("Find Template")
            .build_and_send()
            .await
            .unwrap_or_default();

        wrapper(move |content: &mut C| {
            let target = selected.and_then(|pos| items.get(pos).cloned());
            if target.is_some() {
                content.filter = None;
            }
            content.jump_target.set(target);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn finder_target_is_resolved_at_render_and_a_removed_target_does_not_jump() {
        let content = Template {
            items: vec!["first".into(), "second".into()],
            ..Template::default()
        };
        let mut state = ListState::default().with_selected(Some(0));
        let mut terminal = Terminal::new(TestBackend::new(60, 10)).unwrap();
        content.jump_target.set(Some("second".into()));
        terminal
            .draw(|frame| content.render(frame, frame.area(), &mut state, true))
            .unwrap();
        assert_eq!(state.selected(), Some(1));

        content.jump_target.set(Some("removed".into()));
        terminal
            .draw(|frame| content.render(frame, frame.area(), &mut state, true))
            .unwrap();
        assert_eq!(state.selected(), Some(1));
    }
}
