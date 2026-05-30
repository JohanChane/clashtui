use crate::functions::command::{check_config, edit, test_config};
use crate::functions::file::profile::{db, select, update_profile};
use crate::functions::restful::download;
use crate::tui::widget::popmsg::Confirm;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, atomic::Ordering};

use ratatui::style::Style;

use super::*;

/// Traffic usage info parsed from subscription-userinfo header.
#[derive(Debug, Clone, Default)]
pub struct TrafficInfo {
    pub upload: u64,
    pub download: u64,
    pub total: u64,
    pub expire: u64,
}


static TRAFFIC_CACHE: std::sync::LazyLock<Mutex<HashMap<String, TrafficInfo>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Parse subscription-userinfo header value into TrafficInfo.
/// Format: "upload=123; download=456; total=789; expire=1234567890"
fn parse_traffic_info(header_value: &str) -> TrafficInfo {
    let mut info = TrafficInfo::default();
    for part in header_value.split(';') {
        let part = part.trim();
        if let Some((key, value)) = part.split_once('=') {
            let value: u64 = value.trim().parse().unwrap_or(0);
            match key.trim() {
                "upload" => info.upload = value,
                "download" => info.download = value,
                "total" => info.total = value,
                "expire" => info.expire = value,
                _ => {}
            }
        }
    }
    info
}

pub fn fetch_traffic_for_url(url: &str, with_proxy: bool) {
    let url = url.to_owned();
    std::thread::spawn(
        move || match download::fetch_subscription_userinfo(&url, with_proxy) {
            Ok(Some(userinfo)) => {
                let info = parse_traffic_info(&userinfo);
                TRAFFIC_CACHE.lock().unwrap().insert(url, info);
            }
            _ => {}
        },
    );
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{size:.0} {unit}", size = size, unit = UNITS[unit_idx])
    } else {
        format!("{size:.1} {unit}", size = size, unit = UNITS[unit_idx])
    }
}

fn traffic_percentage(used: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        (used as f64 / total as f64 * 100.0).min(100.0)
    }
}

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
        ([KeyCode::Enter], Key::Select, "Select"),
        (
            [KeyCode::Char('i')],
            Key::Action(Action::Add),
            "Import (URL or file)"
        ),
        (
            [KeyCode::Char('d'), KeyCode::Char('d')],
            Key::Action(Action::Delete),
            "Delete profile"
        ),
        ([KeyCode::Char('e')], Key::Action(Action::Edit), "Edit"),
        (
            [KeyCode::Char('p')],
            Key::Action(Action::Preview),
            "Preview"
        ),
        ([KeyCode::Char('u')], Key::Action(Action::Update), "Update"),
        (
            [KeyCode::Char('/')],
            Key::Action(Action::Search),
            "Search/Filter"
        ),
        ([KeyCode::Char('t')], Key::Action(Action::Test), "Test"),
        (
            [KeyCode::Char('c')],
            Key::Action(Action::Check),
            "Check config"
        ),
        (
            [KeyCode::Char('C'), KeyCode::Char('u')],
            Key::Action(Action::CopyUrl),
            "Copy URL"
        ),
        (
            [KeyCode::Char('f')],
            Key::Action(Action::FzfFind),
            "Find profile"
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
            key("P"),
            Key::Action(Action::ToggleNoPp),
            "Toggle no proxy-provider"
        ),
        (
            key("O"),
            Key::Action(Action::ToggleUpdateWithProxy),
            "Toggle update with proxy"
        ),
        (
            key("n"),
            Key::Action(Action::Traffic),
            "Show traffic"
        ),
    ]
);

#[derive(Clone, Copy)]
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
                        "Add" => Ok(Key::Action(Action::Add)),
                        "ImportFile" => Ok(Key::Action(Action::ImportFile)),
                        "Delete" => Ok(Key::Action(Action::Delete)),
                        "Edit" => Ok(Key::Action(Action::Edit)),
                        "Preview" => Ok(Key::Action(Action::Preview)),
                        "Update" => Ok(Key::Action(Action::Update)),
                        "UpdateAll" => Ok(Key::Action(Action::UpdateAll)),
                        "Search" => Ok(Key::Action(Action::Search)),
                        "Test" => Ok(Key::Action(Action::Test)),
                        "Check" => Ok(Key::Action(Action::Check)),
                        "CopyUrl" => Ok(Key::Action(Action::CopyUrl)),
                        "FzfFind" => Ok(Key::Action(Action::FzfFind)),
                        "GoTop" => Ok(Key::Action(Action::GoTop)),
                        "GoEnd" => Ok(Key::Action(Action::GoEnd)),
                        "ToggleNoPp" => Ok(Key::Action(Action::ToggleNoPp)),
                        "ToggleUpdateWithProxy" => Ok(Key::Action(Action::ToggleUpdateWithProxy)),
                        "Traffic" => Ok(Key::Action(Action::Traffic)),
                        s => Err(de::Error::unknown_variant(
                            s,
                            &[
                                "Add",
                                "ImportFile",
                                "Delete",
                                "Edit",
                                "Preview",
                                "Update",
                                "UpdateAll",
                                "Search",
                                "Test",
                                "Check",
                                "CopyUrl",
                                "FzfFind",
                                "GoTop",
                                "GoEnd",
                                "ToggleNoPp",
                                "ToggleUpdateWithProxy",
                                "Traffic",
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

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
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
    Check,
    CopyUrl,
    FzfFind,
    GoTop,
    GoEnd,
    ToggleNoPp,
    ToggleUpdateWithProxy,
    Traffic,
}

impl TryFrom<&crate::tui::Key> for Key {
    type Error = ();

    fn try_from(value: &crate::tui::Key) -> Result<Self, Self::Error> {
        let agent = agent();
        if !agent.is_empty() {
            return agent.get(value).map(|act| *act).ok_or(());
        }

        Ok(match value.code {
            KeyCode::Right | KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('l') => {
                Self::Switch
            }
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
                    let pf = tri!(db::get(&name).unwrap().load_local_profile());
                    tri!(check_config(&pf.path));
                    tri!(select(db::get(&name).unwrap()).await);
                    sync!((Self, Self::Mate))
                }
                .spawn_at(task_set);
            }
            Key::Action(action) => match action {
                Action::GoTop => state.select_first(),
                Action::GoEnd => state.select_last(),
                Action::Traffic => {
                    let name = get_name!(self, state);
                    actions::show_traffic(name).spawn_at(task_set);
                }
                Action::FzfFind => {
                    let items = self.items.clone();
                    actions::fzf_find(items).spawn_at(task_set)
                }
                Action::Add | Action::ImportFile => action.act(String::new()).spawn_at(task_set),
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

        let current = &crate::config::CONFIG
            .data
            .lock()
            .unwrap()
            .get_current()
            .map(|pf| pf.name)
            .unwrap_or_default();
        let spinner_chars = ['/', '-', '\\', '|'];
        let spinner_idx = (crate::tui::app::SPINNER_FRAME.load(Ordering::Relaxed) as usize / 8) % 4;

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
                    spans.push(Span::raw("* ").style(section.border));
                } else {
                    spans.push(Span::raw("  "));
                }

                spans.push(Span::raw(value.as_str()));
                spans.push(Span::raw(" "));
                spans.push(Span::raw(extra.as_str()).style(section.muted));

                ListItem::new(Line::from(spans))
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
                Self::Search => search().await,
                Self::Add | Self::ImportFile => import().await,
                Self::Edit => _edit(name).await,
                Self::Delete => delete(name).await,
                Self::Preview => preview(name).await,
                Self::Update => update(name).await,
                Self::Test => test(name).await,
                Self::Check => check(name).await,
                Self::CopyUrl => copy_url(name).await,
                Self::ToggleNoPp => toggle_no_pp(name).await,
                Self::ToggleUpdateWithProxy => toggle_update_with_proxy(name).await,
                Self::Traffic => {
                    unreachable!("traffic handled in handle_key_event directly")
                }
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
        let is_singbox = crate::config::CONFIG.core_type() == crate::config::CoreType::Singbox;

        if is_singbox {
            let content: serde_json::Value = if is_url {
                let mut response =
                    tri!(crate::functions::restful::download::profile(&source, false));
                tri!(serde_json::from_reader(&mut response))
            } else {
                let file = tri!(std::fs::File::open(&source));
                tri!(serde_json::from_reader(file))
            };
            let path = crate::functions::file::PROFILE_JSONS_PATH.join(format!("{name}.json"));
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
            let path = crate::functions::file::PROFILE_YAMLS_PATH.join(format!("{name}.yaml"));
            {
                let mut response =
                    tri!(crate::functions::restful::download::profile(&source, false));
                let content: serde_yml::Mapping = tri!(serde_yml::from_reader(&mut response));
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

    async fn toggle_update_with_proxy(name: String) -> CB {
        tri!(db::toggle_update_with_proxy(&name));

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
        let with_proxy = db::get(&name).map(|pf| pf.update_with_proxy).unwrap_or(false);
        // Fetch traffic info before updating
        if let Some(pf) = db::get(&name) {
            if let crate::config::database::ProfileType::Url(ref url) = pf.dtype {
                fetch_traffic_for_url(url, with_proxy);
            }
        }
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
            let with_proxy = db::get(name).map(|pf| pf.update_with_proxy).unwrap_or(false);
            let result = update_profile(db::get(name).unwrap(), with_proxy).await;
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

    pub(super) async fn show_traffic(name: String) -> CB {
        use crate::config::database::ProfileType;
        use crate::functions::file::net_resource::{ExtractNetResources, ResourceSection};

        let pf = db::get(&name);
        let with_proxy = pf.as_ref().map(|p| p.update_with_proxy).unwrap_or(false);

        let mut lines = Vec::new();
        let mut urls_to_fetch: Vec<(String, String)> = Vec::new();

        if let Some(ref p) = pf {
            if let ProfileType::Url(ref url) = p.dtype {
                let cached = {
                    let cache = TRAFFIC_CACHE.lock().unwrap();
                    cache.get(url).cloned()
                };
                if let Some(info) = cached {
                    let domain = crate::functions::file::profile::extract_domain(url)
                        .unwrap_or(url);
                    let used = info.upload + info.download;
                    let total_str = if info.total == 0 {
                        "unlimited".to_owned()
                    } else {
                        format!(
                            "{} ({:.0}%)",
                            human_bytes(info.total),
                            traffic_percentage(used, info.total)
                        )
                    };
                    lines.push(format!("[{name}] {domain}"));
                    lines.push(format!(
                        "  [Used: {} / {}]",
                        human_bytes(used),
                        total_str
                    ));
                } else {
                    urls_to_fetch.push((name.clone(), url.clone()));
                }
            }
        }

        let mut pp_urls: Vec<(String, String)> = Vec::new();

        if let Ok(groups) = crate::functions::file::template::read_profile_ppg(&name) {
            for providers in groups.values() {
                for (pp_name, pp_url) in providers {
                    if !pp_urls.iter().any(|(_, u)| u == pp_url) {
                        pp_urls.push((pp_name.clone(), pp_url.clone()));
                    }
                }
            }
        }

        let profile_yaml_path =
            crate::functions::file::PROFILE_YAMLS_PATH.join(format!("{name}.yaml"));
        if let Ok(content) = std::fs::read_to_string(&profile_yaml_path) {
            if let Ok(mapping) = serde_yml::from_str::<serde_yml::Mapping>(&content) {
                for resource in mapping.extract(&[ResourceSection::ProxyProvider]) {
                    if !pp_urls.iter().any(|(_, u)| u == &resource.url) {
                        pp_urls.push((resource.name.clone(), resource.url.clone()));
                    }
                }
            }
        }

        let profile_json_path =
            crate::functions::file::PROFILE_JSONS_PATH.join(format!("{name}.json"));
        if let Ok(content) = std::fs::read_to_string(&profile_json_path) {
            if let Ok(mapping) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(pp_map) = mapping.get("proxy-providers") {
                    if let Some(obj) = pp_map.as_object() {
                        for (pp_name, pp_val) in obj {
                            if let Some(pp_url) = pp_val.get("url").and_then(|v| v.as_str()) {
                                let url = pp_url.to_string();
                                if !pp_urls.iter().any(|(_, u)| u == &url) {
                                    pp_urls.push((pp_name.clone(), url));
                                }
                            }
                        }
                    }
                }
            }
        }

        for (pp_name, pp_url) in &pp_urls {
            if !urls_to_fetch.iter().any(|(_, u)| u == pp_url) {
                urls_to_fetch.push((pp_name.clone(), pp_url.clone()));
            }
        }

        let mut fetch_handles = Vec::with_capacity(urls_to_fetch.len());
        for (entry_name, entry_url) in &urls_to_fetch {
            let entry_url_c = entry_url.clone();
            let entry_name_c = entry_name.clone();
            let wp = with_proxy;
            fetch_handles.push(tokio::task::spawn_blocking(move || {
                match download::fetch_subscription_userinfo(&entry_url_c, wp) {
                    Ok(Some(userinfo)) => Some((entry_name_c, entry_url_c, userinfo)),
                    _ => None,
                }
            }));
        }

        for handle in fetch_handles {
            if let Ok(Some((entry_name, entry_url, userinfo))) = handle.await {
                let info = parse_traffic_info(&userinfo);
                {
                    let mut cache = TRAFFIC_CACHE.lock().unwrap();
                    cache.insert(entry_url.clone(), info.clone());
                }
                let domain =
                    crate::functions::file::profile::extract_domain(&entry_url).unwrap_or(&entry_url);
                let used = info.upload + info.download;
                let total_str = if info.total == 0 {
                    "unlimited".to_owned()
                } else {
                    format!(
                        "{} ({:.0}%)",
                        human_bytes(info.total),
                        traffic_percentage(used, info.total)
                    )
                };
                if !lines.is_empty() {
                    lines.push(String::new());
                }
                lines.push(format!("[{entry_name}] {domain}"));
                lines.push(format!(
                    "  [Used: {} / {}]",
                    human_bytes(used),
                    total_str
                ));
            }
        }

        if lines.is_empty() {
            lines.push("No traffic data available.".to_owned());
        }

        Confirm::dismiss_any("Traffic".to_owned())
            .with_prompt(lines.join("\n"))
            .build_and_send();

        do_nothing()
    }

    async fn test(name: String) -> CB {
        let pf = tri!(db::get(name).unwrap().load_local_profile());
        let result = test_config(Some(&pf.path), false);
        Confirm::title("Test Result".to_owned())
            .with_prompt(result)
            .build_and_send();

        do_nothing()
    }

    async fn check(name: String) -> CB {
        let pf = tri!(db::get(name).unwrap().load_local_profile());
        match check_config(&pf.path) {
            Ok(()) => {
                Confirm::title("Check Passed".to_owned())
                    .with_prompt("Configuration is valid.".to_owned())
                    .build_and_send();
            }
            Err(e) => Confirm::err(e),
        }

        do_nothing()
    }

    async fn copy_url(name: String) -> CB {
        use crate::config::database::ProfileType;

        let url = match db::get(&name) {
            Some(pf) => match &pf.dtype {
                ProfileType::Url(url) => url.clone(),
                _ => {
                    Confirm::title("Not a URL profile".to_owned())
                        .with_prompt(format!("'{name}' is not a URL profile."))
                        .build_and_send();
                    return do_nothing();
                }
            },
            None => {
                Confirm::title("Profile not found".to_owned())
                    .with_prompt(format!("Profile '{name}' not found."))
                    .build_and_send();
                return do_nothing();
            }
        };

        let result = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("echo -n '{}' | xclip -selection clipboard 2>/dev/null || echo -n '{}' | wl-copy 2>/dev/null", url, url))
            .output();

        match result {
            Ok(out) if out.status.success() => {
                Confirm::title("Copied".to_owned())
                    .with_prompt(format!("URL copied to clipboard: {url}"))
                    .build_and_send();
            }
            _ => Confirm::err(anyhow::anyhow!(
                "Failed to copy to clipboard. Install xclip or wl-copy."
            )),
        }

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
            let update_with_proxy = pf.update_with_proxy;
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
            let pxy_str = if update_with_proxy { "proxy" } else { "" };
            (name, format!("{domain}|{atime}|{no_pp_str}|{pxy_str}"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_traffic_info_all_fields() {
        let header = "upload=1000; download=2000; total=5000; expire=1234567890";
        let info = parse_traffic_info(header);
        assert_eq!(info.upload, 1000);
        assert_eq!(info.download, 2000);
        assert_eq!(info.total, 5000);
        assert_eq!(info.expire, 1234567890);
    }

    #[test]
    fn parse_traffic_info_partial_fields() {
        let header = "upload=500; download=800";
        let info = parse_traffic_info(header);
        assert_eq!(info.upload, 500);
        assert_eq!(info.download, 800);
        assert_eq!(info.total, 0);
        assert_eq!(info.expire, 0);
    }

    #[test]
    fn parse_traffic_info_empty() {
        let info = parse_traffic_info("");
        assert_eq!(info.upload, 0);
        assert_eq!(info.download, 0);
        assert_eq!(info.total, 0);
        assert_eq!(info.expire, 0);
    }

    #[test]
    fn parse_traffic_info_with_spaces() {
        let header = " upload = 1024 ; download = 2048 ; total = 4096 ";
        let info = parse_traffic_info(header);
        assert_eq!(info.upload, 1024);
        assert_eq!(info.download, 2048);
        assert_eq!(info.total, 4096);
    }

    #[test]
    fn parse_traffic_info_unknown_key_ignored() {
        let header = "upload=100; foo=bar; download=200";
        let info = parse_traffic_info(header);
        assert_eq!(info.upload, 100);
        assert_eq!(info.download, 200);
    }

    #[test]
    fn parse_traffic_info_invalid_number_defaults_zero() {
        let header = "upload=abc; download=200";
        let info = parse_traffic_info(header);
        assert_eq!(info.upload, 0);
        assert_eq!(info.download, 200);
    }

    #[test]
    fn parse_traffic_info_missing_equals() {
        let header = "upload=100; invalid; download=200";
        let info = parse_traffic_info(header);
        assert_eq!(info.upload, 100);
        assert_eq!(info.download, 200);
    }

    #[test]
    fn traffic_percentage_normal() {
        // 50 used of 200 total = 25%
        assert!((traffic_percentage(50, 200) - 25.0).abs() < 0.01);
        // 100 used of 100 total = 100%
        assert!((traffic_percentage(100, 100) - 100.0).abs() < 0.01);
        // 0 used of 100 total = 0%
        assert!((traffic_percentage(0, 100) - 0.0).abs() < 0.01);
    }

    #[test]
    fn traffic_percentage_zero_total() {
        assert_eq!(traffic_percentage(100, 0), 0.0);
        assert_eq!(traffic_percentage(0, 0), 0.0);
    }

    #[test]
    fn traffic_percentage_capped_at_100() {
        assert_eq!(traffic_percentage(200, 100), 100.0);
        assert_eq!(traffic_percentage(1000, 10), 100.0);
    }

    #[test]
    fn human_bytes_units() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(500), "500 B");
        assert_eq!(human_bytes(1023), "1023 B");
        assert_eq!(human_bytes(1024), "1.0 KB");
        assert_eq!(human_bytes(1536), "1.5 KB");
        assert_eq!(human_bytes(1_048_576), "1.0 MB");
        assert_eq!(human_bytes(1_073_741_824), "1.0 GB");
        assert_eq!(human_bytes(1_099_511_627_776), "1.0 TB");
    }

    /// Simulate the full traffic data flow without real URLs:
    /// parse header, format output, verify the exact display string.
    #[test]
    fn simulated_traffic_display_url_profile() {
        let header = "upload=123456789; download=987654321; total=5368709120";
        let info = parse_traffic_info(header);
        let used = info.upload + info.download;
        // 123456789 + 987654321 = 1111111110 bytes ≈ 1.0 GB
        // total = 5368709120 bytes = 5.0 GB
        // percentage = 1111111110 / 5368709120 * 100 ≈ 20.7%
        let total_str = format!(
            "{} ({:.0}%)",
            human_bytes(info.total),
            traffic_percentage(used, info.total)
        );
        let line = format!("  [Used: {} / {}]", human_bytes(used), total_str);
        assert!(line.contains("1.0 GB"), "expected ~1.0 GB used");
        assert!(line.contains("5.0 GB"), "expected 5.0 GB total");
    }

    /// Simulate unlimited traffic: total = 0, expect "unlimited".
    #[test]
    fn simulated_traffic_display_unlimited() {
        let header = "upload=500000; download=1500000; total=0";
        let info = parse_traffic_info(header);
        let used = info.upload + info.download;
        let total_str = if info.total == 0 {
            "unlimited".to_owned()
        } else {
            format!(
                "{} ({:.0}%)",
                human_bytes(info.total),
                traffic_percentage(used, info.total)
            )
        };
        let line = format!("  [Used: {} / {}]", human_bytes(used), total_str);
        assert!(line.contains("unlimited"), "expected unlimited total");
    }

    /// Simulate proxy-provider traffic from subscription-userinfo header.
    #[test]
    fn simulated_proxy_provider_traffic() {
        let pp_headers = [
            ("pvd0", "upload=0; download=31457280; total=104857600"),
            ("bak", "upload=5242880; download=0; total=0"),
        ];

        let mut lines = Vec::new();
        for (name, header) in &pp_headers {
            let info = parse_traffic_info(header);
            let used = info.upload + info.download;
            let total_str = if info.total == 0 {
                "unlimited".to_owned()
            } else {
                format!(
                    "{} ({:.0}%)",
                    human_bytes(info.total),
                    traffic_percentage(used, info.total)
                )
            };
            lines.push(format!("[{name}] example.com"));
            lines.push(format!(
                "  [Used: {} / {}]",
                human_bytes(used),
                total_str
            ));
        }

        let result = lines.join("\n");
        assert!(result.contains("[pvd0]"));
        assert!(result.contains("[bak]"));
        assert!(result.contains("unlimited")); // total = 0 => unlimited
        assert!(result.contains("100.0 MB")); // pvd0 total = 100 MB
    }

    /// Simulate all cached: profile traffic from cache, proxy-provider from header.
    #[test]
    fn simulated_full_traffic_popup() {
        // Simulate profile traffic (from cache)
        let pf_header = "upload=500000000; download=2500000000; total=10737418240";
        let pf_info = parse_traffic_info(pf_header);
        let pf_used = pf_info.upload + pf_info.download;
        // pf_used = 3,000,000,000 ≈ 2.8 GB, total = 10 GB

        // Simulate proxy-provider traffic
        let pp_headers = [
            ("hajimi", "upload=100000000; download=200000000; total=5368709120"),
            ("mojie", "upload=50000000; download=0; total=0"),
        ];

        let mut lines = Vec::new();

        // Profile entry
        {
            let total_str = format!(
                "{} ({:.0}%)",
                human_bytes(pf_info.total),
                traffic_percentage(pf_used, pf_info.total)
            );
            lines.push("[mojie-profile] sub.example.com".to_string());
            lines.push(format!(
                "  [Used: {} / {}]",
                human_bytes(pf_used),
                total_str
            ));
        }

        for (name, header) in &pp_headers {
            let info = parse_traffic_info(header);
            let used = info.upload + info.download;
            let total_str = if info.total == 0 {
                "unlimited".to_owned()
            } else {
                format!(
                    "{} ({:.0}%)",
                    human_bytes(info.total),
                    traffic_percentage(used, info.total)
                )
            };
            lines.push(String::new());
            lines.push(format!("[{name}] pp.example.com"));
            lines.push(format!(
                "  [Used: {} / {}]",
                human_bytes(used),
                total_str
            ));
        }

        let result = lines.join("\n");
        assert!(result.contains("[mojie-profile]"));
        assert!(result.contains("10.0 GB")); // total 10 GB
        assert!(result.contains("[hajimi]"));
        assert!(result.contains("5.0 GB")); // total 5 GB
        assert!(result.contains("[mojie]"));
        assert!(result.contains("unlimited")); // mojie PP unlimited
    }
}
