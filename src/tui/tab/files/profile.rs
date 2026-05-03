use crate::functions::command::{edit, test_config};
use crate::functions::file::profile::{db, select, update_profile};
use crate::tui::widget::popmsg::Confirm;

use super::*;

mod_agent!(
    Key,
    [
        ([KeyCode::Left], Key::Switch, ""),
        ([KeyCode::Right], Key::Switch, ""),
        ([KeyCode::Down], Key::MoveDown, ""),
        ([KeyCode::Up], Key::MoveUp, ""),
        ([KeyCode::Enter], Key::Action(Action::Apply), ""),
        ([KeyCode::Char('i')], Key::Action(Action::Add), ""),
        ([KeyCode::Char('e')], Key::Action(Action::Edit), ""),
        ([KeyCode::Char('d')], Key::Action(Action::Delete), ""),
        ([KeyCode::Char('p')], Key::Action(Action::Preview), ""),
        ([KeyCode::Char('u')], Key::Action(Action::Update), ""),
        ([KeyCode::Char('/')], Key::Action(Action::Search), ""),
        ([KeyCode::Char('t')], Key::Action(Action::Test), ""),
        ([KeyCode::Char('g'), KeyCode::Char('g')], Key::Action(Action::GoTop), "Go to top"),
        ([KeyCode::Char('g'), KeyCode::Char('e')], Key::Action(Action::GoEnd), "Go to end"),
    ]
);

#[derive(Clone, Copy, serde::Deserialize)]
pub enum Key {
    Switch,
    MoveUp,
    MoveDown,
    Select,

    Action(Action),
}

#[derive(Clone, Copy, serde::Deserialize)]
pub enum Action {
    Add,
    Apply,
    Delete,
    Edit,
    Preview,
    Update,
    Search,
    Test,
    GoTop,
    GoEnd,
}

impl TryFrom<&KeyEvent> for Key {
    type Error = ();

    fn try_from(value: &KeyEvent) -> Result<Self, Self::Error> {
        let agent = agent();
        if !agent.is_empty() {
            return agent.get(value).map(|act| *act).ok_or(());
        }

        if value.kind != crossterm::event::KeyEventKind::Press {
            return Err(());
        }
        Ok(match value.code {
            KeyCode::Right | KeyCode::Left => Self::Switch,
            KeyCode::Down => Self::MoveDown,
            KeyCode::Up => Self::MoveUp,
            KeyCode::Enter => Self::Select,

            _ => return Err(()),
        })
    }
}

#[derive(Default)]
pub struct Profile {
    items: Vec<String>,
    // atime: Vec<Option<Duration>>,
    atime: Vec<String>,
    filter: Option<String>,
}

impl BasicTabContent for Profile {
    type Key = Key;
    type State = ListState;

    const TITLE: &str = "Profile";

    fn all_shortcuts() -> &'static [(KeyCombo, Self::Key, &'static str)] {
        all_shortcuts()
    }
}

impl DualTabContent for Profile {
    type Mate = super::template::Template;

    fn init(&mut self, task_set: &mut FutureSet<(Self, Self::Mate)>, _: &mut Self::State) {
        async { sync!((Self, Self::Mate)) }.spawn_at(task_set);
    }

    fn handle_key_event(
        &mut self,
        key: Self::Key,
        task_set: &mut FutureSet<(Self, Self::Mate)>,
        state: &mut Self::State,
    ) -> bool {
        match key {
            Key::Switch => return true,
            Key::MoveUp => state.select_previous(),
            Key::MoveDown => state.select_next(),

            Key::Select => {
                let name = get_name!(self, state);
                async move {
                    let pf = tri!(db::get(&name).unwrap().load_local_profile());
                    let atime = pf
                        .atime()
                        .map(display_duration)
                        .unwrap_or_else(|| "Unknown".to_string());
                    let domain = pf
                        .dtype
                        .get_domain()
                        .unwrap_or_else(|| "Unknown".to_string());

                    let info = format!("Profile:{name}\nAtime:{atime}\nDomain:{domain}");

                    let action: Action = todo!("crate::tui::popmsg::SelectSingle");

                    action.act(name).await
                }
                .spawn_at(task_set);
            }
            Key::Action(action) => {
                match action {
                    Action::GoTop => state.select_first(),
                    Action::GoEnd => state.select_last(),
                    _ => {
                        let name = get_name!(self, state);
                        action.act(name).spawn_at(task_set);
                        return false;
                    }
                }
            }
        }
        false
    }

    fn render(&self, f: &mut Frame, area: Rect, state: &mut Self::State, is_focused: bool) {
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
            .zip(self.atime.iter())
            // filter content now
            .filter(|(value, _)| self.filter.as_deref().is_none_or(|pat| value.contains(pat)))
            .map(|(value, extra)| {
                ListItem::new(Line::from(vec![
                    Span::raw(value),
                    Span::raw("("),
                    Span::raw(extra).style(Theme::get().profile_tab.update_interval),
                    Span::raw(")"),
                ]))
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
                Self::Search => search().await,
                Self::Add => add().await,
                Self::Edit => _edit(name).await,
                Self::Apply => apply(name).await,
                Self::Delete => delete(name).await,
                Self::Preview => preview(name).await,
                Self::Update => update(name).await,
                Self::Test => test(name).await,
                Self::GoTop | Self::GoEnd => do_nothing(),
            }
        }
    }

    type CB = Box<dyn for<'a> FnOnce(&'a mut C) + Send + 'static>;
    type C = (Profile, <Profile as DualTabContent>::Mate);

    async fn search() -> CB {
        let filter = tri!(
            Input::new()
                .with_title("Filter".to_owned())
                .build_and_send()
                .await,
            or_cancel
        );

        wrapper(|(content, _): &mut C| {
            content.filter = (!filter.is_empty()).then_some(filter);
        })
    }

    async fn add() -> CB {
        let name = tri!(
            Input::new()
                .with_title("Name".to_owned())
                .build_and_send()
                .await,
            or_cancel
        );
        let url = tri!(
            Input::new()
                .with_title("Url".to_owned())
                .build_and_send()
                .await,
            or_cancel
        );
        let pf = tri!(db::create(name, url));
        tri!(update_profile(pf, false, false).await);

        sync!(C)
    }

    async fn _edit(name: String) -> CB {
        let pf = tri!(db::get(name).unwrap().load_local_profile());
        tri!(edit(pf.path.to_str().unwrap()));

        do_nothing()
    }

    async fn apply(name: String) -> CB {
        tri!(select(db::get(name).unwrap()));

        do_nothing()
    }

    async fn delete(name: String) -> CB {
        let pf = db::get(name).unwrap();
        tri!(db::remove(pf));

        sync!(C)
    }

    async fn preview(name: String) -> CB {
        let mut lines = Vec::with_capacity(512);
        let pf = tri!(db::get(name).unwrap().load_local_profile());
        lines.push(
            pf.dtype
                .get_domain()
                .unwrap_or("Imported local file".to_owned()),
        );
        lines.push(Default::default());

        let content = tri!(std::fs::read_to_string(pf.path));
        if content.is_empty() {
            lines.push("yaml file is empty. Please update it.".to_owned());
        } else {
            lines.extend(content.lines().map(|s| s.to_owned()));
        }

        Confirm::title("Preview".to_owned())
            .with_prompt(lines.join("\n"))
            .build_and_send();

        do_nothing()
    }

    async fn update(name: String) -> CB {
        let with_proxy = todo!("crate::tui::popmsg::SelectSingle");
        let remove_proxy_provider = todo!("crate::tui::popmsg::SelectSingle");
        let result =
            tri!(update_profile(db::get(name).unwrap(), with_proxy, remove_proxy_provider,).await);

        sync!(C)
    }

    async fn test(name: String) -> CB {
        let enable_geodata_mode = todo!("crate::tui::popmsg::SelectSingle");
        let pf = tri!(db::get(name).unwrap().load_local_profile());
        let result = test_config(Some(&pf.path), enable_geodata_mode);
        Confirm::title("Test Result".to_owned())
            .with_prompt(result)
            .build_and_send();

        do_nothing()
    }
}

pub(super) fn get_profiles_with_readable_atime() -> (Vec<String>, Vec<String>) {
    let mut composed: Vec<(String, String)> = crate::functions::file::profile::db::get_all()
        .into_iter()
        .map(|pf| {
            (
                pf.name.clone(),
                pf.load_local_profile()
                    .ok()
                    .and_then(|lp| lp.atime())
                    .map(display_duration)
                    .unwrap_or_else(|| "Unknown".to_owned()),
            )
        })
        .collect();
    composed.sort_unstable();
    let (name, atime) = composed.into_iter().unzip();
    (name, atime)
}

pub(super) fn sync_helper(content: &mut Profile, name: Vec<String>, atime: Vec<String>) {
    content.atime = atime;
    content.items = name;
}

fn display_duration(t: std::time::Duration) -> String {
    use std::time::Duration;
    if t.is_zero() {
        "Just Now".to_string()
    } else if t < Duration::from_secs(60 * 59) {
        let min = t.as_secs() / 60;
        format!("In {} mins", min + 1)
    } else if t < Duration::from_secs(3600 * 24) {
        let hou = t.as_secs() / 3600;
        format!("In {hou} hours")
    } else {
        let day = t.as_secs() / (3600 * 24);
        format!("In about {day} days")
    }
}
