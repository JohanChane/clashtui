use crate::functions::command::{edit, test_config};
use crate::functions::file::profile::{db, select, update_profile};
use crate::tui::widget::popmsg::Confirm;
use std::cell::Cell;
use std::collections::HashSet;
use std::sync::atomic::Ordering;

use super::*;

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
        ([KeyCode::Enter], Key::Select, ""),
        ([KeyCode::Char('i')], Key::Action(Action::Add), "Import (URL or file)"),
        ([KeyCode::Char('e')], Key::Action(Action::Edit), ""),
        ([KeyCode::Char('d'), KeyCode::Char('d')], Key::Action(Action::Delete), "Delete profile"),
        ([KeyCode::Char('p')], Key::Action(Action::Preview), ""),
        ([KeyCode::Char('u')], Key::Action(Action::Update), ""),
        ([KeyCode::Char('a'), KeyCode::Char('u')], Key::Action(Action::UpdateAll), "Update all"),
        ([KeyCode::Char('/')], Key::Action(Action::Search), ""),
        ([KeyCode::Char('t')], Key::Action(Action::Test), ""),
        ([KeyCode::Char('f')], Key::Action(Action::FzfFind), "Fuzzy find profile"),
        ([KeyCode::Char('g'), KeyCode::Char('g')], Key::Action(Action::GoTop), "Go to top"),
        ([KeyCode::Char('G')], Key::Action(Action::GoEnd), "Go to end"),
        ([KeyCode::Char('N')], Key::Action(Action::ToggleNoPp), "Toggle no proxy-provider"),
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
    ImportFile,
    Delete,
    Edit,
    Preview,
    Update,
    UpdateAll,
    Search,
    Test,
    FzfFind,
    GoTop,
    GoEnd,
    ToggleNoPp,
}

impl TryFrom<&crate::tui::Key> for Key {
    type Error = ();

    fn try_from(value: &crate::tui::Key) -> Result<Self, Self::Error> {
        let agent = agent();
        if !agent.is_empty() {
            return agent.get(value).map(|act| *act).ok_or(());
        }

        Ok(match value.code {
            KeyCode::Right | KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('l') => Self::Switch,
            KeyCode::Down | KeyCode::Char('j') => Self::MoveDown,
            KeyCode::Up | KeyCode::Char('k') => Self::MoveUp,
            KeyCode::Enter => Self::Select,

            _ => return Err(()),
        })
    }
}

#[derive(Default)]
pub struct Profile {
    items: Vec<String>,
    atime: Vec<String>,
    filter: Option<String>,
    updating: HashSet<String>,
    jump_target: Cell<Option<usize>>,
}

impl BasicTabContent for Profile {
    type Key = Key;
    type State = ListState;

    const TITLE: &str = "Profile";

    fn all_shortcuts() -> &'static [(KeyCombo, Self::Key, &'static str)] {
        agent::all_shortcuts()
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
                    tri!(select(db::get(&name).unwrap()).await);
                    sync!((Self, Self::Mate))
                }
                .spawn_at(task_set);
            }
            Key::Action(action) => match action {
                Action::GoTop => state.select_first(),
                Action::GoEnd => state.select_last(),
                Action::FzfFind => {
                    let items = self.items.clone();
                    actions::fzf_find(items).spawn_at(task_set)
                }
                Action::Add | Action::ImportFile => {
                    action.act(String::new()).spawn_at(task_set)
                }
                Action::UpdateAll => {
                    for name in &self.items {
                        self.updating.insert(name.clone());
                    }
                    actions::update_all(self.items.clone()).spawn_at(task_set)
                }
                _ => {
                    let name = get_name!(self, state);
                    if matches!(action, Action::Update) {
                        self.updating.insert(name.clone());
                    }
                    action.act(name).spawn_at(task_set)
                }
            },
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

        let current = &crate::config::CONFIG
            .data
            .lock()
            .unwrap()
            .get_current()
            .map(|pf| pf.name)
            .unwrap_or_default();
        let spinner_chars = ['/', '-', '\\', '|'];
        let spinner_idx =
            (crate::tui::app::SPINNER_FRAME.load(Ordering::Relaxed) as usize / 8) % 4;

        let iter = self
            .items
            .iter()
            .zip(self.atime.iter())
            .filter(|(value, _)| self.filter.as_deref().is_none_or(|pat| value.contains(pat)))
            .map(|(value, extra)| {
                let mut spans = Vec::with_capacity(5);

                if self.updating.contains(value.as_str()) {
                    spans.push(Span::raw(format!("{} ", spinner_chars[spinner_idx])));
                } else if value == current.as_str() {
                    spans.push(
                        Span::raw("* ").style(Theme::get().tab.tab_focused),
                    );
                } else {
                    spans.push(Span::raw("  "));
                }

                spans.push(Span::raw(value.as_str()));
                spans.push(Span::raw(" "));
                spans.push(Span::raw(extra.as_str()).style(Theme::get().profile_tab.update_interval));

                ListItem::new(Line::from(spans))
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
                Self::Add | Self::ImportFile => import().await,
                Self::Edit => _edit(name).await,
                Self::Delete => delete(name).await,
                Self::Preview => preview(name).await,
                Self::Update => update(name).await,
                Self::Test => test(name).await,
                Self::ToggleNoPp => toggle_no_pp(name).await,
                Self::FzfFind => unreachable!("FzfFind handled directly"),
                Self::GoTop | Self::GoEnd => do_nothing(),
                Self::UpdateAll => unreachable!("UpdateAll handled directly in handle_key_event"),
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

    pub(super) async fn fzf_find(items: Vec<String>) -> CB {
        let selected = tokio::task::spawn_blocking(move || {
            crate::tui::widget::fzffind::run_fzf(&items, "Find Profile")
        })
        .await
        .unwrap_or(None);

        wrapper(move |(content, _): &mut C| {
            content.jump_target.set(selected);
        })
    }

    async fn import() -> CB {
        let name = tri!(
            Input::new()
                .with_title("Name".to_owned())
                .build_and_send()
                .await,
            or_cancel
        );
        let source = tri!(
            Input::new()
                .with_title("URL or File Path".to_owned())
                .build_and_send()
                .await,
            or_cancel
        );

        let is_url = source.starts_with("http://") || source.starts_with("https://");
        let is_singbox =
            crate::config::CONFIG.cfg_file.core_type == crate::config::CoreType::Singbox;

        if is_singbox {
            let content: serde_json::Value = if is_url {
                let mut response =
                    tri!(crate::functions::restful::download::profile(&source, false));
                tri!(serde_json::from_reader(&mut response))
            } else {
                let file = tri!(std::fs::File::open(&source));
                tri!(serde_json::from_reader(file))
            };
            let path = crate::functions::file::PROFILE_JSONS_PATH
                .join(format!("{name}.json"));
            {
                if let Some(parent) = path.parent() {
                    tri!(std::fs::create_dir_all(parent));
                }
                tri!(std::fs::create_dir_all(
                    &*crate::functions::file::PROFILE_JSONS_PATH
                ));
                let file = tri!(std::fs::File::create(&path));
                tri!(serde_json::to_writer(file, &content));
            }
            {
                let mut pm = crate::config::CONFIG.data.lock().unwrap();
                let dtype = if is_url {
                    crate::config::database::ProfileType::Url(source.clone())
                } else {
                    crate::config::database::ProfileType::Singbox
                };
                pm.insert(&name, dtype);
                tri!(pm.to_file());
            }
        } else if is_url {
            let path =
                crate::functions::file::PROFILE_YAMLS_PATH.join(format!("{name}.yaml"));
            {
                let mut response =
                    tri!(crate::functions::restful::download::profile(&source, false));
                let content: serde_yml::Mapping =
                    tri!(serde_yml::from_reader(&mut response));
                if let Some(parent) = path.parent() {
                    tri!(std::fs::create_dir_all(parent));
                }
                let file = tri!(std::fs::File::create(&path));
                tri!(serde_yml::to_writer(file, &content));
            }
            tri!(db::create(name, source));
        } else {
            tri!(crate::functions::file::profile::import_profile_from_file(
                &source, &name
            ));
        }

        sync!(C)
    }

    async fn _edit(name: String) -> CB {
        let pf = tri!(db::get(name).unwrap().load_local_profile());
        tri!(edit(pf.path.to_str().unwrap()));

        do_nothing()
    }

    async fn toggle_no_pp(name: String) -> CB {
        {
            let pf = tri!(db::get(&name).ok_or_else(|| anyhow::anyhow!("Profile not found")));
            if pf.dtype == crate::config::database::ProfileType::Singbox
                || crate::config::CONFIG
                    .data
                    .lock()
                    .unwrap()
                    .contains_in_singbox(&pf.name)
            {
                Confirm::err(anyhow::anyhow!(
                    "no_pp is not applicable for sing-box profiles (proxy-provider not supported)"
                ));
                return do_nothing();
            }
        }
        tri!(db::toggle_no_pp(&name));

        let (names, atime) = get_profiles_with_readable_atime();
        wrapper(move |(content, _): &mut C| {
            sync_helper(content, names, atime);
        })
    }

    async fn delete(name: String) -> CB {
        let rx = Confirm::title(format!("Delete profile?"))
            .with_prompt(format!("Delete {name}?\nEnter to confirm, Esc to cancel"))
            .build_and_send();
        if rx.await.is_err() {
            return do_nothing();
        }

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
        let with_proxy = false;
        let result = update_profile(db::get(&name).unwrap(), with_proxy).await;

        let (names, atime) = get_profiles_with_readable_atime();
        wrapper(move |(content, _): &mut C| {
            content.updating.remove(&name);
            match result {
                Ok(upd) => {
                    sync_helper(content, names, atime);
                    let mut msg = format!("Updated: {}", upd.name);
                    if !upd.net_updates.is_empty() {
                        msg.push('\n');
                        msg.push_str(&crate::functions::file::net_resource::format_net_updates(
                            &upd.net_updates,
                        ));
                    }
                    Confirm::title("Updated".to_owned())
                        .with_prompt(msg)
                        .build_and_send();
                }
                Err(e) => Confirm::err(e),
            }
        })
    }

    pub(super) async fn update_all(names: Vec<String>) -> CB {
        let mut results = Vec::with_capacity(names.len());
        for name in &names {
            let result = update_profile(db::get(name).unwrap(), false).await;
            results.push((name.clone(), result));
        }

        let (new_names, new_atime) = get_profiles_with_readable_atime();
        wrapper(move |(content, _): &mut C| {
            let mut msgs = Vec::with_capacity(results.len());
            let mut has_net_updates = false;
            for (name, result) in results {
                content.updating.remove(&name);
                match result {
                    Ok(upd) => {
                        let mut msg = format!("Updated: {}", upd.name);
                        if !upd.net_updates.is_empty() {
                            has_net_updates = true;
                            msg.push('\n');
                            msg.push_str(
                                &crate::functions::file::net_resource::format_net_updates(
                                    &upd.net_updates,
                                ),
                            );
                        }
                        msgs.push(msg);
                    }
                    Err(e) => msgs.push(format!("{name}: {e}")),
                }
            }
            sync_helper(content, new_names, new_atime);
            let has_errors = msgs.iter().any(|m| m.contains(':') && !m.contains(": ok"));
            let title = if has_errors {
                "Updated (some failed)"
            } else {
                "All Updated"
            };
            Confirm::title(title.to_owned())
                .with_prompt(msgs.join("\n"))
                .build_and_send();
        })
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
    use crate::config::database::ProfileType;
    use crate::functions::file::profile::extract_domain;

    let mut composed: Vec<(String, String)> = crate::functions::file::profile::db::get_all()
        .into_iter()
        .map(|pf| {
            let name = pf.name.clone();
            let no_pp = pf.no_pp;
            let is_singbox = pf.dtype == ProfileType::Singbox
                || crate::config::CONFIG
                    .data
                    .lock()
                    .unwrap()
                    .contains_in_singbox(&pf.name);
            let domain = match &pf.dtype {
                ProfileType::File => "local import".to_owned(),
                ProfileType::Url(url) => extract_domain(url).unwrap_or("unknown").to_owned(),
                ProfileType::Singbox => "singbox profile".to_owned(),
                ProfileType::Template { .. } => "template".to_owned(),
            };
            let atime = pf
                .load_local_profile()
                .ok()
                .and_then(|lp| lp.atime())
                .map(display_duration)
                .unwrap_or_else(|| "Unknown".to_owned());
            let no_pp_str = if is_singbox {
                "N/A"
            } else if no_pp {
                "nopp"
            } else {
                ""
            };
            (name, format!("{domain}|{atime}|{no_pp_str}"))
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
