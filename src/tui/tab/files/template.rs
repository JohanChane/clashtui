use super::*;
use crate::functions::command::edit;
use crate::functions::file::template::*;
use crate::tui::widget::popmsg::Confirm;
use std::cell::Cell;

mod_agent!(
    Key,
    [
        ([KeyCode::Left], Key::Switch, ""),
        ([KeyCode::Right], Key::Switch, ""),
        ([KeyCode::Char('h')], Key::Switch, ""),
        ([KeyCode::Char('l')], Key::Switch, ""),
        ([KeyCode::Down], Key::MoveDown, ""),
        ([KeyCode::Up], Key::MoveUp, ""),
        ([KeyCode::Char('j')], Key::MoveDown, ""),
        ([KeyCode::Char('k')], Key::MoveUp, ""),
        ([KeyCode::Char('d'), KeyCode::Char('d')], Key::Action(Action::Delete), "Delete template"),
        ([KeyCode::Char('e')], Key::Action(Action::Edit), ""),
        ([KeyCode::Char('E')], Key::Action(Action::EditProviders), "Edit providers file"),
        ([KeyCode::Char('p')], Key::Action(Action::Preview), ""),
        ([KeyCode::Enter], Key::Action(Action::Generate), ""),
        ([KeyCode::Char('f')], Key::Action(Action::FzfFind), "Find template"),
        ([KeyCode::Char('g'), KeyCode::Char('g')], Key::Action(Action::GoTop), "Go to top"),
        ([KeyCode::Char('G')], Key::Action(Action::GoEnd), "Go to end"),
        ([KeyCode::Char('/')], Key::Action(Action::Search), "Search/Filter"),
    ]
);

#[derive(Clone, Copy, Debug, serde::Deserialize)]
pub enum Action {
    Generate,
    Delete,
    Edit,
    EditProviders,
    Preview,
    Search,
    FzfFind,
    GoTop,
    GoEnd,
}

#[derive(Clone, Copy, Debug, serde::Deserialize)]
pub enum Key {
    Switch,
    MoveUp,
    MoveDown,
    Select,

    Action(Action),
}

impl TryFrom<&crate::tui::Key> for Key {
    type Error = ();

    fn try_from(value: &crate::tui::Key) -> Result<Self, Self::Error> {
        let agent = agent();
        if !agent.is_empty() {
            return agent.get(value).map(|act| *act).ok_or(());
        }

        Ok(match value.code {
            KeyCode::Enter => Self::Select,
            KeyCode::Right | KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('l') => Self::Switch,
            KeyCode::Down | KeyCode::Char('j') => Self::MoveDown,
            KeyCode::Up | KeyCode::Char('k') => Self::MoveUp,

            _ => return Err(()),
        })
    }
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

    fn all_shortcuts() -> &'static [(KeyCombo, Self::Key, &'static str)] {
        agent::all_shortcuts()
    }
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
        log::debug!("Template::handle_key_event: key={key:?} items.len={}", self.items.len());
        match key {
            Key::Switch => return true,
            Key::MoveDown => state.select_next(),
            Key::MoveUp => state.select_previous(),

            Key::Select => todo!(),

            Key::Action(action) => {
                log::debug!("Template::Action: {action:?}");
                match action {
                    Action::GoTop => state.select_first(),
                    Action::GoEnd => state.select_last(),
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

        let block = Block::bordered()
            .border_style(if is_focused {
                Theme::get().tab.tab_focused
            } else {
                Theme::get().tab.dualtab_unfocused
            })
            .title(Self::TITLE);

        let block = if let Some(filter) = self.filter.as_ref() {
            block.title_bottom(Line::raw(format!(" {filter} ")).right_aligned().reversed())
        } else {
            block.title_bottom(Line::raw(format!(" /: Search ")).right_aligned().reversed())
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
                Theme::get().tab.item_highlighted
            } else {
                Theme::get().tab.item_unhighlighted
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
                Self::EditProviders => _edit_providers().await,
                Self::Preview => preview(name).await,
                Self::Search => search().await,
                Self::FzfFind => unreachable!("FzfFind handled directly"),
                Self::GoTop | Self::GoEnd => do_nothing(),
            }
        }
    }

    type CB = Box<dyn for<'a> FnOnce(&'a mut C) + Send + 'static>;
    type C = (<Template as DualTabContentMate>::Mate, Template);

    async fn generate(name: String) -> CB {
        let profile_name = format!("{name}.tpl");
        let groups = tri!(read_template_proxy_providers());
        let is_singbox = crate::config::CONFIG.core_type() == crate::config::CoreType::Singbox;
        if is_singbox {
            tri!(apply_template_singbox(&name, &profile_name, &groups, false, false).await);
        } else {
            tri!(apply_template(&name, &profile_name, &groups));
        }
        sync!(C)
    }

    async fn delete(name: String) -> CB {
        let rx = Confirm::title(format!("Delete template?"))
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

    async fn _edit_providers() -> CB {
        let path = match crate::config::CONFIG.core_type() {
            crate::config::CoreType::Mihomo => crate::config::template_proxy_providers_path(),
            crate::config::CoreType::Singbox => crate::config::singbox_template_proxy_providers_path(),
        };
        log::debug!("template::_edit_providers: path={}", path.display());
        tri!(edit(path.to_str().unwrap()));
        do_nothing()
    }

    async fn preview(name: String) -> CB {
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
        let selected = tokio::task::spawn_blocking(move || {
            crate::tui::widget::fzffind::run_fzf(&items, "Find Template")
        })
        .await
        .unwrap_or(None);

        wrapper(move |(_, content): &mut C| {
            content.jump_target.set(selected);
        })
    }
}
