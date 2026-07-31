use super::*;
use crate::functions::command::edit;
use crate::functions::file::template::*;
use ratatui::style::Style;
use std::cell::Cell;

key_map!(
    Key,
    FileMap::new()
        .with_common([
            (KeyCode::Left, Key::Switch),
            (KeyCode::Right, Key::Switch),
            (KeyCode::Char('h'), Key::Switch),
            (KeyCode::Char('l'), Key::Switch),
            (KeyCode::Down, Key::MoveDown),
            (KeyCode::Up, Key::MoveUp),
            (KeyCode::Char('j'), Key::MoveDown),
            (KeyCode::Char('k'), Key::MoveUp),
            (KeyCode::Char('D'), Key::Action(Action::Delete)),
            (KeyCode::Char('e'), Key::Action(Action::Edit)),
            (KeyCode::Char('E'), Key::Action(Action::EditProviders)),
            (KeyCode::Char('p'), Key::Action(Action::Preview)),
            (KeyCode::Enter, Key::Action(Action::Generate)),
            (KeyCode::Char('f'), Key::Action(Action::FzfFind)),
            (KeyCode::Char('G'), Key::GoEnd),
            (KeyCode::Char('/'), Key::Action(Action::Search)),
        ])
        .with_submap(
            "Nav",
            KeyCode::Char('g'),
            [(KeyCode::Char('g'), Key::GoTop)]
        )
);

#[derive_aliases::derive(..Key, Debug)]
pub enum Key {
    Switch,
    MoveUp,
    MoveDown,
    Select,
    GoTop,
    GoEnd,

    Action(Action),
}
impl AsStaticStr for Key {
    fn as_static_str(&self) -> &'static str {
        use crate::tui::key::consts::*;
        match self {
            Self::Switch => "Switch panel",
            Self::MoveUp => MOVE_UP,
            Self::MoveDown => MOVE_DOWN,
            Self::Select => "Select",
            Self::GoTop => GO_TOP,
            Self::GoEnd => GO_BOTTOM,
            Self::Action(Action::Generate) => "Generate",
            Self::Action(Action::Delete) => "Delete",
            Self::Action(Action::Edit) => "Edit",
            Self::Action(Action::EditProviders) => "Edit proxy providers",
            Self::Action(Action::Preview) => "Preview",
            Self::Action(Action::Search) => FILTER,
            Self::Action(Action::FzfFind) => "Find template",
        }
    }
}

#[derive_aliases::derive(..Action, Debug)]
pub enum Action {
    Generate,
    Delete,
    Edit,
    EditProviders,
    Preview,
    Search,
    FzfFind,
}

#[derive(Default)]
pub struct Template {
    items: Vec<String>,
    filter: Option<String>,
    jump_target: Cell<Option<usize>>,
}

impl BasicTabContent for Template {
    type Key = Key;
    type State = ListState;

    const TITLE: &str = "Template";
}

impl DualTabContentMate for Template {
    type Mate = super::profile::Profile;

    fn init(&mut self, task_set: &mut FutureSet<(Self::Mate, Self)>, _: &mut Self::State) {
        async {
            let templates = tri!(get_all_templates());
            wrapper(|(_, content): &mut (Self::Mate, Self)| content.items = templates)
        }
        .spawn_at(task_set);
    }

    fn handle_key_event(
        &mut self,
        key: Self::Key,
        task_set: &mut FutureSet<(Self::Mate, Self)>,
        state: &mut Self::State,
    ) -> bool {
        log::debug!(
            "Template::handle_key_event: key={key:?} items.len={}",
            self.items.len()
        );
        match key {
            Key::Switch => return true,
            Key::MoveDown => state.select_next(),
            Key::MoveUp => state.select_previous(),
            Key::GoTop => state.select_first(),
            Key::GoEnd => state.select_last(),

            Key::Select => todo!(),

            Key::Action(action) => {
                log::debug!("Template::Action: {action:?}");
                match action {
                    Action::FzfFind => {
                        let items = self.items.clone();
                        actions::fzf_find(items).spawn_at(task_set);
                        return false;
                    }
                    Action::EditProviders => {
                        action.act(String::new()).spawn_at(task_set);
                        return false;
                    }
                    _ => {
                        let name = get_name!(self, state);
                        log::debug!("Template::Action name={name}");
                        action.act(name).spawn_at(task_set);
                        return false;
                    }
                }
            }
        }
        false
    }

    fn render(&self, f: &mut Frame, area: Rect, state: &mut Self::State, is_focused: bool) {
        if let Some(idx) = self.jump_target.take() {
            state.select(Some(idx));
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
        let block = if let Some(submap_name) = km::get_submap_name() {
            block.title_bottom(submap_name)
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

mod actions {
    use super::*;

    impl Action {
        pub async fn act(self, name: String) -> CB {
            match self {
                Self::Generate => generate(name).await,
                Self::Delete => delete(name).await,
                Self::Edit => _edit(name).await,
                Self::EditProviders => _edit_providers(name).await,
                Self::Preview => preview(name).await,
                Self::Search => search().await,
                Self::FzfFind => unreachable!("FzfFind handled directly"),
            }
        }
    }

    type CB = Box<dyn for<'a> FnOnce(&'a mut C) + Send + 'static>;
    type C = (<Template as DualTabContentMate>::Mate, Template);

    async fn generate(name: String) -> CB {
        let profile_name = format!("{name}.tpl");
        let is_singbox = crate::config::CONFIG.core_type() == crate::config::CoreType::Singbox;
        if is_singbox {
            tri!(apply_template_singbox(&name, &profile_name, false, false).await);
        } else {
            tri!(apply_template(&name, &profile_name));
        }
        sync!(C)
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
        wrapper(move |(_, content): &mut C| {
            content.items = templates;
        })
    }

    async fn _edit(name: String) -> CB {
        let path = crate::functions::file::TEMPLATE_PATH.join(&name);
        log::debug!("template::_edit: path={}", path.display());
        tri!(edit(path.to_str().unwrap()));
        do_nothing()
    }

    async fn _edit_providers(_name: String) -> CB {
        let subdir = match crate::config::CONFIG.core_type() {
            crate::config::CoreType::Mihomo => "mihomo",
            crate::config::CoreType::Singbox => "sing-box",
        };
        let path = crate::config::config_dir_path()
            .join(subdir)
            .join("template_proxy_providers.yaml");
        log::debug!("template::_edit_providers: path={}", path.display());
        tri!(edit(path.to_str().unwrap()));
        do_nothing()
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

        wrapper(|(_, content): &mut C| {
            content.filter = (!filter.is_empty()).then_some(filter);
        })
    }

    pub(super) async fn fzf_find(items: Vec<String>) -> CB {
        let selected = FzfFinder::new(items)
            .with_title("Find Template")
            .build_and_send()
            .await
            .unwrap_or_default();

        wrapper(move |(_, content): &mut C| {
            content.jump_target.set(selected);
        })
    }
}
