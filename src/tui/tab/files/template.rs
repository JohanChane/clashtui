use super::*;
use crate::functions::command::edit;
use crate::functions::file::template::*;
use ratatui::style::Style;
use std::cell::Cell;

mod_agent!(
    keymap,
    crate::tui::binding::Scope::FileTemplate,
    Key,
    [
        ([KeyCode::Left], Key::Switch, "Switch pane"),
        ([KeyCode::Right], Key::Switch, "Switch pane"),
        ([KeyCode::Char('h')], Key::Switch, "Switch pane"),
        ([KeyCode::Char('l')], Key::Switch, "Switch pane"),
        ([KeyCode::Down], Key::MoveDown, "Move down"),
        ([KeyCode::Up], Key::MoveUp, "Move up"),
        ([KeyCode::Char('j')], Key::MoveDown, "Move down"),
        ([KeyCode::Char('k')], Key::MoveUp, "Move up"),
        (
            [KeyCode::Char('d'), KeyCode::Char('d')],
            Key::Delete,
            "Delete template"
        ),
        ([KeyCode::Char('e')], Key::Edit, "Edit"),
        (
            [KeyCode::Char('E')],
            Key::EditProviders,
            "Edit proxy providers"
        ),
        ([KeyCode::Char('p')], Key::Preview, "Preview"),
        ([KeyCode::Enter], Key::Generate, "Generate"),
        ([KeyCode::Char('f')], Key::FzfFind, "Find template"),
        (
            [KeyCode::Char('g'), KeyCode::Char('g')],
            Key::GoTop,
            "Go to top"
        ),
        ([KeyCode::Char('G')], Key::GoEnd, "Go to end"),
        ([KeyCode::Char('/')], Key::Search, "Search/Filter"),
    ]
);

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Key {
    Switch,
    MoveUp,
    MoveDown,
    Select,

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

impl Key {
    /// Bridge to the internal dispatch-only `Action` enum for variants that
    /// have async `act()` handlers. Returns `None` for directly-handled
    /// variants (GoTop, GoEnd, FzfFind).
    fn to_action(self) -> Option<Action> {
        match self {
            Key::Generate => Some(Action::Generate),
            Key::Delete => Some(Action::Delete),
            Key::Edit => Some(Action::Edit),
            Key::EditProviders => Some(Action::EditProviders),
            Key::Preview => Some(Action::Preview),
            Key::Search => Some(Action::Search),
            // Directly handled in handle_key_event -- no act() needed
            Key::Switch
            | Key::MoveUp
            | Key::MoveDown
            | Key::Select
            | Key::GoTop
            | Key::GoEnd
            | Key::FzfFind => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

    fn keymap() -> &'static crate::tui::binding::Keymap<Self::Key> {
        keymap::get()
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
        log::debug!(
            "Template::handle_key_event: key={key:?} items.len={}",
            self.items.len()
        );
        match key {
            Key::Switch => return true,
            Key::MoveDown => state.select_next(),
            Key::MoveUp => state.select_previous(),

            Key::Select => todo!(),

            Key::GoTop => state.select_first(),
            Key::GoEnd => state.select_last(),

            Key::FzfFind => {
                let items = self.items.clone();
                actions::fzf_find(items).spawn_at(task_set);
                return false;
            }
            Key::EditProviders => {
                key.to_action()
                    .unwrap()
                    .act(String::new())
                    .spawn_at(task_set);
                return false;
            }
            _ => {
                let name = get_name!(self, state);
                log::debug!("Template::Action name={name}");
                key.to_action().unwrap().act(name).spawn_at(task_set);
                return false;
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
                Self::GoTop | Self::GoEnd => do_nothing(),
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
