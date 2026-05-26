use super::*;
use crate::functions::command::edit;
use crate::functions::file::template::*;
use crate::tui::widget::popmsg::Confirm;
use ratatui::style::Style;
use std::cell::Cell;

mod_agent!(
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
            Key::Action(Action::Delete),
            "Delete template"
        ),
        ([KeyCode::Char('e')], Key::Action(Action::Edit), "Edit"),
        (
            [KeyCode::Char('E')],
            Key::Action(Action::EditProviders),
            "Edit proxy providers"
        ),
        (
            [KeyCode::Char('p')],
            Key::Action(Action::Preview),
            "Preview"
        ),
        ([KeyCode::Enter], Key::Action(Action::Generate), "Generate"),
        (
            [KeyCode::Char('f')],
            Key::Action(Action::FzfFind),
            "Find template"
        ),
        (
            [KeyCode::Char('g'), KeyCode::Char('g')],
            Key::Action(Action::GoTop),
            "Go to top"
        ),
        (
            [KeyCode::Char('G')],
            Key::Action(Action::GoEnd),
            "Go to end"
        ),
        (
            [KeyCode::Char('/')],
            Key::Action(Action::Search),
            "Search/Filter"
        ),
    ]
);

#[derive(Clone, Copy, Debug)]
pub enum Key {
    Switch,
    MoveUp,
    MoveDown,
    Select,

    Action(Action),
}

impl serde::Serialize for Key {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Key::Switch => serializer.serialize_str("Switch"),
            Key::MoveUp => serializer.serialize_str("MoveUp"),
            Key::MoveDown => serializer.serialize_str("MoveDown"),
            Key::Select => serializer.serialize_str("Select"),
            Key::Action(action) => {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("Action", action)?;
                map.end()
            }
        }
    }
}

impl<'de> serde::Deserialize<'de> for Key {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor};
        use std::fmt;

        struct KeyVisitor;

        impl<'de> Visitor<'de> for KeyVisitor {
            type Value = Key;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a string (unit variant) or mapping (Action: <name>)")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Key, E> {
                match v {
                    "Switch" => Ok(Key::Switch),
                    "MoveUp" => Ok(Key::MoveUp),
                    "MoveDown" => Ok(Key::MoveDown),
                    "Select" => Ok(Key::Select),
                    s => Err(de::Error::unknown_variant(
                        s,
                        &["Switch", "MoveUp", "MoveDown", "Select", "Action: ..."],
                    )),
                }
            }

            fn visit_map<M: de::MapAccess<'de>>(self, mut map: M) -> Result<Key, M::Error> {
                let k: String = map
                    .next_key()?
                    .ok_or_else(|| de::Error::missing_field("variant"))?;
                if k == "Action" {
                    let v: String = map.next_value()?;
                    match v.as_str() {
                        "Generate" => Ok(Key::Action(Action::Generate)),
                        "Delete" => Ok(Key::Action(Action::Delete)),
                        "Edit" => Ok(Key::Action(Action::Edit)),
                        "EditProviders" => Ok(Key::Action(Action::EditProviders)),
                        "Preview" => Ok(Key::Action(Action::Preview)),
                        "Search" => Ok(Key::Action(Action::Search)),
                        "FzfFind" => Ok(Key::Action(Action::FzfFind)),
                        "GoTop" => Ok(Key::Action(Action::GoTop)),
                        "GoEnd" => Ok(Key::Action(Action::GoEnd)),
                        s => Err(de::Error::unknown_variant(
                            s,
                            &[
                                "Generate",
                                "Delete",
                                "Edit",
                                "EditProviders",
                                "Preview",
                                "Search",
                                "FzfFind",
                                "GoTop",
                                "GoEnd",
                            ],
                        )),
                    }
                } else {
                    Err(de::Error::unknown_field(&k, &["Action"]))
                }
            }
        }

        deserializer.deserialize_any(KeyVisitor)
    }
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
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

impl TryFrom<&crate::tui::Key> for Key {
    type Error = ();

    fn try_from(value: &crate::tui::Key) -> Result<Self, Self::Error> {
        let agent = agent();
        if !agent.is_empty() {
            return agent.get(value).map(|act| *act).ok_or(());
        }

        Ok(match value.code {
            KeyCode::Enter => Self::Select,
            KeyCode::Right | KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('l') => {
                Self::Switch
            }
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
        log::debug!(
            "Template::handle_key_event: key={key:?} items.len={}",
            self.items.len()
        );
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
