use super::*;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use tab::prelude::*;
use tokio::sync::Notify;
use widget::help::HelpPanel;
use widget::popmsg::PopUp;

// 50fps
const TICK_RATE: std::time::Duration = std::time::Duration::from_millis(20);
pub(super) static FULL_RENDER: Notify = Notify::const_new();
pub(super) static SPINNER_FRAME: AtomicU8 = AtomicU8::new(0);
pub(crate) static QUIT: AtomicBool = AtomicBool::new(false);

#[derive_aliases::derive(..Key)]
pub enum AppKey {
    TabNext,
    TabPrev,
    Quit,
    Help,
    #[serde(untagged)]
    Tab(TabTarget),
    #[serde(untagged)]
    Global(GlobalAction),
}

#[derive_aliases::derive(..Action, Debug)]
#[repr(u8)]
pub enum TabTarget {
    #[serde(rename = "TabStatus")]
    Status,
    #[serde(rename = "TabFile")]
    File,
    #[serde(rename = "TabProxies")]
    Proxies,
    #[serde(rename = "TabConnections")]
    Connections,
    #[serde(rename = "TabLogs")]
    Logs,
    #[serde(rename = "TabSettings")]
    Settings,
    #[serde(rename = "TabCoreSrvCtl")]
    CoreSrvCtl,
}

#[derive_aliases::derive(..Action, Debug)]
pub enum GlobalAction {
    OpenCoreDataDir,
    OpenCoreInstallDir,
    RestartCoreService,
    CloseAllConnections,
}

impl GlobalAction {
    fn execute(self) -> anyhow::Result<()> {
        use crate::{
            config::{CONFIG, CoreType},
            functions::{command, restful::connection},
        };
        match self {
            Self::OpenCoreDataDir => {
                let dir = crate::config::core_data_dir(CONFIG.core_type());
                command::open_dir(&dir)
            }
            Self::OpenCoreInstallDir => {
                let dir = match CONFIG.core_type() {
                    CoreType::Mihomo => &CONFIG.cfg_file.mihomo.core.config_dir,
                    CoreType::Singbox => &CONFIG.cfg_file.singbox.core.config_dir,
                };
                let dir = std::path::Path::new(dir);
                command::open_dir(dir.parent().unwrap_or(dir))
            }
            Self::RestartCoreService => {
                let output = command::restart_service()?;
                anyhow::ensure!(!output.starts_with("Error"), "{output}");
                Ok(())
            }
            Self::CloseAllConnections => Ok(connection::terminate_all_connections()?),
        }
    }
}

pub(in crate::tui) mod km {
    //! Global shortcuts use the same configuration and help generation as page shortcuts.
    use super::{AppKey, GlobalAction, TabTarget};
    use crate::tui::key::{Document, Key, instancing::defaults::DefaultMap};
    use crossterm::event::{KeyCode, KeyModifiers};

    key_map!(AppKey, super::defaults());

    pub(super) fn defaults() -> DefaultMap<AppKey> {
        DefaultMap::new()
            .with_common(
                [
                    TabTarget::Status,
                    TabTarget::File,
                    TabTarget::Proxies,
                    TabTarget::Connections,
                    TabTarget::Logs,
                    TabTarget::Settings,
                    TabTarget::CoreSrvCtl,
                ]
                .into_iter()
                .enumerate()
                .map(|(index, tab)| {
                    (
                        KeyCode::Char((b'1' + index as u8) as char),
                        AppKey::Tab(tab),
                    )
                })
                .chain(APP_KEYS),
            )
            .with_submap(
                "Global",
                Key {
                    code: KeyCode::Char('g'),
                    modifiers: KeyModifiers::CONTROL,
                },
                [
                    (
                        KeyCode::Char('c'),
                        AppKey::Global(GlobalAction::OpenCoreDataDir),
                    ),
                    (
                        KeyCode::Char('m'),
                        AppKey::Global(GlobalAction::OpenCoreInstallDir),
                    ),
                    (
                        KeyCode::Char('f'),
                        AppKey::Global(GlobalAction::RestartCoreService),
                    ),
                    (
                        KeyCode::Char('t'),
                        AppKey::Global(GlobalAction::CloseAllConnections),
                    ),
                ],
            )
    }

    pub use km::set;

    pub fn get_docs() -> crate::tui::key::KeyDesc {
        km::get_docs()
    }
    pub(super) use km::{cancel_pending, get_submap_name, pending_keys};

    pub(super) fn handles_chord(key: &Key) -> bool {
        pending_keys().is_some() || km::starts_chord(key)
    }

    pub(super) fn route_key(tab: &mut impl super::TuiTab, key: &Key) -> Option<AppKey> {
        super::route_key(tab, key, km::get(), &mut km::SUBMAP.lock().unwrap())
    }

    pub(super) fn tab_titles(titles: impl IntoIterator<Item = &'static str>) -> Vec<String> {
        super::tab_titles(titles, km::get())
    }

    const APP_KEYS: [(KeyCode, AppKey); 4] = [
        (KeyCode::Tab, AppKey::TabNext),
        (KeyCode::BackTab, AppKey::TabPrev),
        (KeyCode::Char('q'), AppKey::Quit),
        (KeyCode::Char('?'), AppKey::Help),
    ];
    impl Document for AppKey {
        fn get_doc(&self) -> &'static str {
            match self {
                Self::Tab(tab) => match tab {
                    TabTarget::Status => "Switch to Status",
                    TabTarget::File => "Switch to File",
                    TabTarget::Proxies => "Switch to Proxies",
                    TabTarget::Connections => "Switch to Connections",
                    TabTarget::Logs => "Switch to Logs",
                    TabTarget::Settings => "Switch to Settings",
                    TabTarget::CoreSrvCtl => "Switch to CoreSrvCtl",
                },
                Self::TabNext => "Tab Next",
                Self::TabPrev => "Tab Previous",
                Self::Quit => "Quit",
                Self::Help => "Help",
                Self::Global(action) => match action {
                    GlobalAction::OpenCoreDataDir => "Open core data dir",
                    GlobalAction::OpenCoreInstallDir => "Open core install dir",
                    GlobalAction::RestartCoreService => "Start core service",
                    GlobalAction::CloseAllConnections => "Close all connections",
                },
            }
        }
    }
}

pub struct App {
    tabs: Vec<Tab>,
    popup: PopUp,
    help: HelpPanel,

    tab_index: u8,
}

impl App {
    fn new() -> Self {
        let mut app = Self {
            tabs: vec![
                StatusTab::default().into(),
                FileTab::default().into(),
                ProxiesTab::default().into(),
                ConnectionsTab::default().into(),
                LogsTab::default().into(),
                SettingsTab::default().into(),
                CoreSrvCtlTab::default().into(),
            ],
            popup: PopUp::default(),
            help: HelpPanel::default(),
            tab_index: 0,
        };
        app.tabs[0].on_enter();
        app
    }
    #[tokio::main]
    pub async fn serve() -> anyhow::Result<()> {
        signals::Signals::start()?;
        let mut app = Self::new();
        let mut events = crossterm::event::EventStream::new();
        let mut invt = tokio::time::interval(TICK_RATE);
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(std::io::stdout()))?;

        while !QUIT.load(Ordering::Relaxed) {
            terminal.draw(|f| app.render(f))?;
            app.sync();

            let ev = {
                use futures_lite::StreamExt as _;
                // this tick here ensures that fps is stable
                let mut tick = Box::pin(invt.tick());
                let ev = tokio::select! {
                    Some(ev) = events.next() => ev?,
                    _ = &mut tick => continue,
                    // if we switch between screens
                    _ = FULL_RENDER.notified() => {
                        // first we hold tui for output
                        FULL_RENDER.notified().await;
                        // then we tell ratatui to re-render everything
                        terminal.clear()?;
                        continue
                    },
                };
                tick.await;
                ev
            };

            use crossterm::event::Event;
            match ev {
                Event::Key(key_event) if key_event.kind.is_press() => {
                    #[cfg(debug_assertions)]
                    the_egg(key_event.code);
                    let key = key_event.into();
                    app.handle_key_event(&key);
                }
                Event::Resize(..) => terminal.autoresize()?,
                _ => (),
            }
        }

        log::trace!("App Exit");
        Ok(())
    }

    /// KeyEvent Route:
    /// PopUp → Help → Pending page/global chords → New chords → Page/global single keys.
    /// Help also allows starting global chords, without activating page bindings.
    fn handle_key_event(&mut self, kv: &Key) {
        log::debug!("K: {kv:?}");

        if self.popup.check() {
            self.cancel_pending();
            self.popup.handle_key_event(kv);
            return;
        }

        if self.help.is_active() {
            self.cancel_pending();
            if km::handles_chord(kv) {
                self.help.dismiss();
                self.handle_global_kv(kv);
            } else {
                self.help.handle_key_event(kv);
            }
            return;
        }

        let ti = self.tab_index as usize;
        if let Some(action) = km::route_key(&mut self.tabs[ti], kv) {
            self.handle_action(action);
        }
    }
    fn render(&mut self, f: &mut ratatui::Frame) {
        use ratatui::prelude::{Constraint, Layout};

        let chunks = Layout::default()
            .constraints([Constraint::Length(3), Constraint::Fill(1)])
            .split(f.area());

        render_tabbar(
            km::tab_titles(self.tabs.iter().map(|tab| tab.title())),
            self.tab_index,
            f,
            chunks[0],
        );

        self.tabs[self.tab_index as usize].render(f, chunks[1]);

        if let Some(hints) = combined_hints(
            km::pending_keys(),
            self.tabs[self.tab_index as usize].pending_keys(),
        ) {
            widget::which::render(f, &hints);
        }

        if self.help.is_active() {
            self.help.render(f, &self.tabs[self.tab_index as usize]);
        }

        if self.popup.check() {
            self.popup.render(f, Default::default());
        }
    }

    fn cancel_pending(&self) {
        km::cancel_pending();
        self.tabs[self.tab_index as usize].cancel_pending();
    }

    fn switch_tab(&mut self, new_index: u8) {
        if new_index != self.tab_index {
            self.cancel_pending();
            self.tabs[self.tab_index as usize].on_leave();
            self.tab_index = new_index;
            self.tabs[self.tab_index as usize].on_enter();
        }
    }

    /// Global bindings while help is open; page bindings are inactive here.
    fn handle_global_kv(&mut self, kv: &Key) -> bool {
        if km::pending_keys().is_none() && !km::km::has_binding(kv) {
            return false;
        }
        if let Ok(key) = AppKey::try_from(kv) {
            self.handle_action(key);
        }
        true
    }

    fn handle_action(&mut self, key: AppKey) {
        match key {
            AppKey::Tab(target) => self.switch_tab(target as u8),
            AppKey::TabNext | AppKey::TabPrev => {
                let count = self.tabs.len();
                let offset = if key == AppKey::TabPrev { count - 1 } else { 1 };
                self.switch_tab(((self.tab_index as usize + offset) % count) as u8);
            }
            AppKey::Quit => {
                QUIT.store(true, Ordering::Relaxed);
            }
            AppKey::Help => self.help.toggle(),
            AppKey::Global(action) => {
                tokio::task::spawn_blocking(move || {
                    if let Err(error) = action.execute() {
                        popmsg::prelude::Confirm::err(format!("{error:#}"));
                    }
                });
            }
        }
    }
    fn sync(&mut self) {
        SPINNER_FRAME.fetch_add(1, Ordering::Relaxed);
        self.popup.sync();
        if self.popup.check() {
            self.cancel_pending();
        }
        self.tabs.iter_mut().for_each(|tab| tab.sync());
    }
}

/// A prefix and its second key are consumed, including a mismatch or Esc.
/// The router considers global candidates before forwarding a pending key here.
fn handle_tab_key(tab: &mut impl TuiTab, key: &Key) -> bool {
    let consumed = tab.pending_keys().is_some() || tab.has_binding(key);
    tab.handle_key_event(key);
    consumed
}

/// Retain both scopes after a shared prefix. Only a complete global chord wins
/// over a page chord; a mismatch never falls through to a standalone binding.
fn route_key<'a>(
    tab: &mut impl TuiTab,
    key: &Key,
    globals: &'a key::KeyMap<AppKey>,
    pending: &mut Option<&'a key::SubMap<AppKey>>,
) -> Option<AppKey> {
    if pending.is_some() || tab.pending_keys().is_some() {
        if pending.is_some()
            && let Some(action) = key::instancing::try_from(key, globals, pending)
        {
            tab.cancel_pending();
            return Some(action);
        }
        if tab.pending_keys().is_some() {
            tab.handle_key_event(key);
        }
        return None;
    }

    if matches!(globals.get(key), Some(key::MaybeMap::SubMap(_))) {
        key::instancing::try_from(key, globals, pending);
        if tab.starts_chord(key) {
            tab.handle_key_event(key);
        }
        return None;
    }

    if handle_tab_key(tab, key) {
        None
    } else {
        key::instancing::try_from(key, globals, pending)
    }
}

fn combined_hints<'a>(
    global: Option<&'a key::WhichHints>,
    page: Option<&'a key::WhichHints>,
) -> Option<std::borrow::Cow<'a, key::WhichHints>> {
    use std::{borrow::Cow, collections::BTreeMap};
    match (global, page) {
        (Some(global), Some(page)) => {
            debug_assert_eq!(global.prefix, page.prefix);
            // Global descriptions win on the same second key, just as dispatch does.
            let keys: BTreeMap<_, _> = page.keys.iter().chain(&global.keys).cloned().collect();
            Some(Cow::Owned(key::WhichHints {
                prefix: global.prefix,
                name: global.name.clone(),
                keys: keys.into_iter().collect(),
            }))
        }
        (Some(hints), None) | (None, Some(hints)) => Some(Cow::Borrowed(hints)),
        (None, None) => None,
    }
}

fn tab_titles(
    titles: impl IntoIterator<Item = &'static str>,
    bindings: &key::KeyMap<AppKey>,
) -> Vec<String> {
    let mut shortcuts: std::collections::HashMap<usize, Vec<String>> = Default::default();
    for (first, binding) in bindings {
        match binding {
            key::MaybeMap::Action(AppKey::Tab(target)) => {
                shortcuts
                    .entry(*target as usize)
                    .or_default()
                    .push(first.to_string());
            }
            key::MaybeMap::SubMap(submap) => {
                for (second, action) in &submap.inner {
                    if let AppKey::Tab(target) = action {
                        shortcuts
                            .entry(*target as usize)
                            .or_default()
                            .push(format!("{first} {second}"));
                    }
                }
            }
            _ => {}
        }
    }
    titles
        .into_iter()
        .enumerate()
        .map(|(index, title)| {
            if let Some(keys) = shortcuts.get_mut(&index) {
                keys.sort();
                format!("{} {title}", keys.join("/"))
            } else {
                title.to_owned()
            }
        })
        .collect()
}

/// each item should represent for one tab
fn render_tabbar(
    titles: impl IntoIterator<Item = String>,
    selected: u8,
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
) {
    use crate::tui::theme::Theme;
    use ratatui::style::Styled;
    use ratatui::widgets::{Block, Tabs};

    let theme = Theme::get();
    let block = Block::bordered().title(" Clashtui ");
    let block = if let Some(submap_name) = km::get_submap_name() {
        block.title_bottom(submap_name)
    } else {
        block
    };
    let titles = titles
        .into_iter()
        .map(|title| title.set_style(theme.tabbar.text));
    let widget = Tabs::new(titles)
        .block(block)
        .highlight_style(theme.tabbar.highlight)
        .select(Some(selected as usize));
    f.render_widget(widget, area);
}

/// Ha! Magic Code!
#[cfg(debug_assertions)]
fn the_egg(key: crossterm::event::KeyCode) {
    use crossterm::event::KeyCode;
    static INSTANCE: std::sync::Mutex<u8> = std::sync::Mutex::new(0);
    let mut current = INSTANCE.lock().unwrap();
    match *current {
        0 | 1 if matches!(key, KeyCode::Up) => (),
        2 | 3 if matches!(key, KeyCode::Down) => (),
        4 | 6 if matches!(key, KeyCode::Left) => (),
        5 | 7 if matches!(key, KeyCode::Right) => (),
        8 | 10 if matches!(key, KeyCode::Char('b') | KeyCode::Char('B')) => (),
        9 | 11 if matches!(key, KeyCode::Char('a') | KeyCode::Char('A')) => (),
        _ => {
            *current = 0;
            return;
        }
    }
    *current += 1;
    if *current == 12 {
        log::debug!("You've found the egg!")
    }
}

#[cfg(test)]
mod which_tests {
    use super::*;
    use crate::tui::key::{self, Document, KeyDesc, KeyMap, SubMap, WhichHints};
    use std::{cell::Cell, sync::OnceLock};

    #[test]
    fn global_chords_match_dev_bindings_and_clear_pending_state() {
        km::set(serde_yml::Value::Null).unwrap();
        let prefix = "<C-g>".parse().unwrap();
        assert!(km::handles_chord(&prefix));
        assert!(!km::handles_chord(&"q".parse().unwrap()));
        for (key, expected, description) in [
            ("c", GlobalAction::OpenCoreDataDir, "Open core data dir"),
            (
                "m",
                GlobalAction::OpenCoreInstallDir,
                "Open core install dir",
            ),
            ("f", GlobalAction::RestartCoreService, "Start core service"),
            (
                "t",
                GlobalAction::CloseAllConnections,
                "Close all connections",
            ),
        ] {
            assert!(AppKey::try_from(&prefix).is_err());
            let hints = km::pending_keys().unwrap();
            assert_eq!(hints.prefix, prefix);
            assert_eq!(hints.keys.len(), 4);
            assert!(hints.keys.contains(&(key.into(), description.into())));
            let actual = AppKey::try_from(&key.parse().unwrap()).ok();
            assert!(matches!(actual, Some(AppKey::Global(action)) if action == expected));
            assert!(km::pending_keys().is_none());
            assert!(km::get_docs().contains(&(format!("<C-g> {key}"), description.into())));
        }
        for key in ["<Esc>", "q", "?", "<Tab>", "1", "x"] {
            assert!(AppKey::try_from(&prefix).is_err());
            assert!(km::handles_chord(&key.parse().unwrap()));
            assert!(AppKey::try_from(&key.parse().unwrap()).is_err());
            assert!(km::pending_keys().is_none());
        }
        assert!(AppKey::try_from(&prefix).is_err());
        km::cancel_pending();
        assert!(km::pending_keys().is_none());
        assert!(matches!(
            AppKey::try_from(&"q".parse().unwrap()),
            Ok(AppKey::Quit)
        ));
        assert!(matches!(
            AppKey::try_from(&"?".parse().unwrap()),
            Ok(AppKey::Help)
        ));
    }

    fn compile_globals(yaml: &str) -> anyhow::Result<(KeyMap<AppKey>, KeyDesc)> {
        key::config::merge(km::defaults(), serde_yml::from_str(yaml)?)
    }

    fn resolve_global(map: &KeyMap<AppKey>, keys: &[&str]) -> Option<AppKey> {
        let mut pending = None;
        let mut action = None;
        for text in keys {
            action = key::instancing::try_from(&text.parse().unwrap(), map, &mut pending);
        }
        action
    }

    #[test]
    fn previous_tab_normalizes_shift_tab_and_supports_rebinding() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let (map, docs) = compile_globals("[]").unwrap();
        for event in [
            KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE),
            KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT),
            KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT),
        ] {
            assert!(matches!(
                key::instancing::try_from(&event.into(), &map, &mut None),
                Some(AppKey::TabPrev)
            ));
        }
        assert!(matches!(
            resolve_global(&map, &["<Tab>"]),
            Some(AppKey::TabNext)
        ));
        assert!(docs.contains(&("<S-Tab>".into(), "Tab Previous".into())));

        let (map, docs) = compile_globals(
            "- {on: '<S-Tab>', action: null}\n- {on: [g, p], action: TabPrev, desc: Previous page}",
        )
        .unwrap();
        assert!(resolve_global(&map, &["<S-Tab>"]).is_none());
        assert!(matches!(
            resolve_global(&map, &["g", "p"]),
            Some(AppKey::TabPrev)
        ));
        assert!(docs.contains(&("g p".into(), "Previous page".into())));
        assert!(!docs.iter().any(|(key, _)| key == "<S-Tab>"));
    }

    #[test]
    fn global_overrides_add_replace_unbind_and_update_hints() {
        let (map, docs) = compile_globals(
            r#"
- {on: ["<C-g>", r], action: RestartCoreService, desc: 重启核心}
- {on: ["<C-g>", f], action: null}
- {on: ["<C-g>", c], action: CloseAllConnections, desc: Close instead}
- {on: "<F1>", action: Help}
- {on: q, action: null}
- {on: Q, action: Quit}
- {on: "1", action: null}
- {on: x, action: TabStatus, desc: Custom status}
"#,
        )
        .unwrap();
        assert!(matches!(
            resolve_global(&map, &["<C-g>", "r"]),
            Some(AppKey::Global(GlobalAction::RestartCoreService))
        ));
        assert!(matches!(
            resolve_global(&map, &["<C-g>", "c"]),
            Some(AppKey::Global(GlobalAction::CloseAllConnections))
        ));
        assert!(matches!(
            resolve_global(&map, &["<C-g>", "m"]),
            Some(AppKey::Global(GlobalAction::OpenCoreInstallDir))
        ));
        assert!(resolve_global(&map, &["<C-g>", "f"]).is_none());
        assert!(resolve_global(&map, &["q"]).is_none());
        assert!(resolve_global(&map, &["1"]).is_none());
        assert!(matches!(resolve_global(&map, &["Q"]), Some(AppKey::Quit)));
        assert!(matches!(
            resolve_global(&map, &["<F1>"]),
            Some(AppKey::Help)
        ));
        assert!(matches!(
            resolve_global(&map, &["x"]),
            Some(AppKey::Tab(TabTarget::Status))
        ));
        assert!(docs.contains(&("x".into(), "Custom status".into())));
        assert!(
            !docs
                .iter()
                .any(|(key, _)| ["1", "1~7", "q", "<C-g> f"].contains(&key.as_str()))
        );
        let key::MaybeMap::SubMap(submap) = &map[&"<C-g>".parse().unwrap()] else {
            panic!("expected chord")
        };
        assert!(submap.hints.keys.contains(&("r".into(), "重启核心".into())));
        assert!(
            submap
                .hints
                .keys
                .contains(&("c".into(), "Close instead".into()))
        );
        assert!(!submap.hints.keys.iter().any(|(key, _)| key == "f"));
    }

    #[test]
    fn all_global_actions_have_plain_names_and_reject_parameters() {
        for (name, expected) in [
            ("Quit", AppKey::Quit),
            ("Help", AppKey::Help),
            ("TabNext", AppKey::TabNext),
            ("TabPrev", AppKey::TabPrev),
            ("TabStatus", AppKey::Tab(TabTarget::Status)),
            ("TabFile", AppKey::Tab(TabTarget::File)),
            ("TabProxies", AppKey::Tab(TabTarget::Proxies)),
            ("TabConnections", AppKey::Tab(TabTarget::Connections)),
            ("TabLogs", AppKey::Tab(TabTarget::Logs)),
            ("TabSettings", AppKey::Tab(TabTarget::Settings)),
            ("TabCoreSrvCtl", AppKey::Tab(TabTarget::CoreSrvCtl)),
            (
                "OpenCoreDataDir",
                AppKey::Global(GlobalAction::OpenCoreDataDir),
            ),
            (
                "OpenCoreInstallDir",
                AppKey::Global(GlobalAction::OpenCoreInstallDir),
            ),
            (
                "RestartCoreService",
                AppKey::Global(GlobalAction::RestartCoreService),
            ),
            (
                "CloseAllConnections",
                AppKey::Global(GlobalAction::CloseAllConnections),
            ),
        ] {
            let (map, _) = compile_globals(&format!("- {{on: '<F2>', action: {name}}}")).unwrap();
            assert!(resolve_global(&map, &["<F2>"]) == Some(expected), "{name}");
        }
        for yaml in [
            "- {on: x, action: Tab1}",
            "- {on: x, action: Tab7}",
            "- {on: x, action: Tab0}",
            "- {on: x, action: Tab8}",
            "- {on: x, action: 'Tab 3'}",
            "- {on: x, action: {Tab: 3}}",
            "- {on: x, action: {Global: RestartCoreService}}",
            "- {on: x, exec: 'echo hello'}",
            "- {on: x, action: Clear}",
            "common: {}",
            "- {on: ['<C-g>'], action: Help}",
            "- {on: x, action: Help}\n- {on: x, action: Quit}",
        ] {
            assert!(compile_globals(yaml).is_err(), "accepted {yaml}");
        }
    }

    #[test]
    fn core_specific_app_scope_overrides_common_and_inherits_defaults() {
        let input = serde_yml::from_str(
            r#"
app: [{on: x, action: Help}]
mihomo:
  app: [{on: y, action: Quit}]
"#,
        )
        .unwrap();
        let mut scopes = key::config::sections(input, "mihomo").unwrap();
        let (map, _) = key::config::merge(km::defaults(), scopes.remove("app").unwrap()).unwrap();
        assert!(resolve_global(&map, &["x"]).is_none());
        assert!(matches!(resolve_global(&map, &["y"]), Some(AppKey::Quit)));
        assert!(matches!(resolve_global(&map, &["q"]), Some(AppKey::Quit)));
    }

    #[derive(Clone, Copy, Eq, PartialEq, serde::Deserialize)]
    enum Action {
        Trigger,
    }
    impl Document for Action {
        fn get_doc(&self) -> &'static str {
            "Trigger"
        }
    }

    struct TestTab {
        pending: Cell<Option<&'static SubMap<Action>>>,
        fired: usize,
    }

    fn map() -> &'static KeyMap<Action> {
        static MAP: OnceLock<KeyMap<Action>> = OnceLock::new();
        MAP.get_or_init(|| key::config::merge(
            key::instancing::defaults::DefaultMap::new(),
            serde_yml::from_str("- {on: m, action: Trigger}\n- {on: x, action: Trigger}\n- {on: [g, g], action: Trigger}\n- {on: [g, q], action: Trigger}\n- {on: [g, '?'], action: Trigger}\n- {on: [g, '<Tab>'], action: Trigger}\n- {on: [q, x], action: Trigger}").unwrap()
        ).unwrap().0)
    }

    impl TuiWidget for TestTab {
        fn handle_key_event(&mut self, key: &Key) {
            if key::instancing::try_from(key, map(), self.pending.get_mut()).is_some() {
                self.fired += 1;
            }
        }
        fn render(&mut self, _: &mut ratatui::Frame, _: ratatui::layout::Rect) {}
        fn sync(&mut self) {}
    }
    impl TuiTab for TestTab {
        fn title(&self) -> &'static str {
            "Test"
        }
        fn key_description(&self) -> KeyDesc {
            vec![]
        }
        fn pending_keys(&self) -> Option<&'static WhichHints> {
            self.pending.get().map(|submap| &submap.hints)
        }
        fn cancel_pending(&self) {
            self.pending.set(None);
        }
        fn has_binding(&self, key: &Key) -> bool {
            map().contains_key(key)
        }
        fn starts_chord(&self, key: &Key) -> bool {
            matches!(map().get(key), Some(key::MaybeMap::SubMap(_)))
        }
    }

    #[test]
    fn chord_prefixes_and_completions_do_not_fall_through_to_global_keys() {
        let mut tab = TestTab {
            pending: Cell::new(None),
            fired: 0,
        };
        for second in ["q", "?", "<Tab>"] {
            assert!(handle_tab_key(&mut tab, &"g".parse().unwrap()));
            assert!(tab.pending_keys().is_some());
            assert!(handle_tab_key(&mut tab, &second.parse().unwrap()));
            assert!(tab.pending_keys().is_none());
        }
        assert_eq!(tab.fired, 3);
        assert!(handle_tab_key(&mut tab, &"q".parse().unwrap()));
        assert!(tab.pending_keys().is_some());
        assert!(handle_tab_key(&mut tab, &"x".parse().unwrap()));
        assert_eq!(tab.fired, 4);
    }

    #[test]
    fn mismatch_and_escape_are_consumed_and_do_not_trigger_standalone_actions() {
        let mut tab = TestTab {
            pending: Cell::new(None),
            fired: 0,
        };
        for second in ["x", "<Esc>", "1"] {
            assert!(handle_tab_key(&mut tab, &"g".parse().unwrap()));
            assert!(handle_tab_key(&mut tab, &second.parse().unwrap()));
            assert!(tab.pending_keys().is_none());
            assert_eq!(tab.fired, 0);
        }
        assert!(handle_tab_key(&mut tab, &"m".parse().unwrap()));
        assert_eq!(tab.fired, 1);
        assert!(!handle_tab_key(&mut tab, &"?".parse().unwrap()));
        assert!(!handle_tab_key(&mut tab, &"<Tab>".parse().unwrap()));
    }

    #[test]
    fn cancelling_a_scope_discards_its_pending_prefix() {
        let mut tab = TestTab {
            pending: Cell::new(None),
            fired: 0,
        };
        handle_tab_key(&mut tab, &"g".parse().unwrap());
        tab.cancel_pending();
        assert!(tab.pending_keys().is_none());
        assert!(!handle_tab_key(&mut tab, &"?".parse().unwrap()));
        assert_eq!(tab.fired, 0);
    }

    #[test]
    fn shared_prefix_keeps_page_and_global_chords_reachable() {
        let (globals, _) =
            compile_globals("- {on: [g, p], action: TabPrev, desc: Previous page}").unwrap();
        let mut pending = None;
        let mut tab = TestTab {
            pending: Cell::new(None),
            fired: 0,
        };

        for second in ["g", "p"] {
            assert!(route_key(&mut tab, &"g".parse().unwrap(), &globals, &mut pending).is_none());
            let hints = combined_hints(pending.map(|s| &s.hints), tab.pending_keys()).unwrap();
            assert!(hints.keys.contains(&("g".into(), "Trigger".into())));
            assert!(hints.keys.contains(&("p".into(), "Previous page".into())));
            assert!(hints.keys.windows(2).all(|pair| pair[0].0 < pair[1].0));
            let action = route_key(&mut tab, &second.parse().unwrap(), &globals, &mut pending);
            if second == "p" {
                assert!(matches!(action, Some(AppKey::TabPrev)));
            } else {
                assert!(action.is_none());
            }
            assert_eq!(tab.fired, 1);
            assert!(pending.is_none());
            assert!(tab.pending_keys().is_none());
        }
    }

    #[test]
    fn identical_chords_run_only_global_action_and_show_its_description() {
        let (globals, _) =
            compile_globals("- {on: [g, q], action: Help, desc: Global help}").unwrap();
        let mut pending = None;
        let mut tab = TestTab {
            pending: Cell::new(None),
            fired: 0,
        };
        route_key(&mut tab, &"g".parse().unwrap(), &globals, &mut pending);
        let hints = combined_hints(pending.map(|s| &s.hints), tab.pending_keys()).unwrap();
        let q: Vec<_> = hints.keys.iter().filter(|(key, _)| key == "q").collect();
        assert_eq!(q, vec![&("q".into(), "Global help".into())]);
        assert!(matches!(
            route_key(&mut tab, &"q".parse().unwrap(), &globals, &mut pending),
            Some(AppKey::Help)
        ));
        assert_eq!(tab.fired, 0);
        assert!(pending.is_none());
        assert!(tab.pending_keys().is_none());
    }

    #[test]
    fn shared_chord_cancellation_never_runs_single_keys_or_starts_another_chord() {
        let (globals, _) = compile_globals("- {on: [g, p], action: TabPrev}").unwrap();
        let mut pending = None;
        let mut tab = TestTab {
            pending: Cell::new(None),
            fired: 0,
        };
        for second in ["<Esc>", "x", "1", "<C-g>"] {
            route_key(&mut tab, &"g".parse().unwrap(), &globals, &mut pending);
            assert!(
                route_key(&mut tab, &second.parse().unwrap(), &globals, &mut pending).is_none()
            );
            assert_eq!(tab.fired, 0);
            assert!(pending.is_none());
            assert!(tab.pending_keys().is_none());
        }
    }

    #[test]
    fn pending_scope_consumes_keys_without_starting_bindings_in_the_other_scope() {
        let (globals, _) = compile_globals("[]").unwrap();
        let mut pending = None;
        let mut tab = TestTab {
            pending: Cell::new(None),
            fired: 0,
        };
        // Page g q must complete instead of executing the global Quit single key.
        route_key(&mut tab, &"g".parse().unwrap(), &globals, &mut pending);
        assert!(route_key(&mut tab, &"q".parse().unwrap(), &globals, &mut pending).is_none());
        assert_eq!(tab.fired, 1);
        // An unmatched global second key must not execute the page's x binding.
        route_key(&mut tab, &"<C-g>".parse().unwrap(), &globals, &mut pending);
        assert!(route_key(&mut tab, &"x".parse().unwrap(), &globals, &mut pending).is_none());
        assert_eq!(tab.fired, 1);
        assert!(pending.is_none());
        assert!(tab.pending_keys().is_none());
    }

    #[test]
    fn chord_prefixes_win_over_single_keys_in_the_other_scope() {
        let (globals, _) =
            compile_globals("- {on: [m, p], action: Help}\n- {on: g, action: Help}").unwrap();
        let mut pending = None;
        let mut tab = TestTab {
            pending: Cell::new(None),
            fired: 0,
        };
        route_key(&mut tab, &"m".parse().unwrap(), &globals, &mut pending);
        assert_eq!(tab.fired, 0);
        assert!(matches!(
            route_key(&mut tab, &"p".parse().unwrap(), &globals, &mut pending),
            Some(AppKey::Help)
        ));
        assert!(route_key(&mut tab, &"g".parse().unwrap(), &globals, &mut pending).is_none());
        assert!(route_key(&mut tab, &"g".parse().unwrap(), &globals, &mut pending).is_none());
        assert_eq!(tab.fired, 1);
    }

    #[test]
    fn tabbar_uses_rebound_keys_and_omits_unbound_shortcuts() {
        let (globals, _) = compile_globals(
            r#"
- {on: "1", action: TabLogs}
- {on: "2", action: null}
- {on: "5", action: null}
- {on: "6", action: null}
- {on: "8", action: TabStatus}
- {on: [z, f], action: TabFile}
"#,
        )
        .unwrap();
        let titles = tab_titles(
            [
                "Status",
                "File",
                "Proxies",
                "Connections",
                "Logs",
                "Settings",
                "CoreSrvCtl",
            ],
            &globals,
        );
        assert_eq!(
            titles,
            [
                "8 Status",
                "z f File",
                "3 Proxies",
                "4 Connections",
                "1 Logs",
                "Settings",
                "7 CoreSrvCtl"
            ]
        );
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 3)).unwrap();
        terminal
            .draw(|f| render_tabbar(titles, 0, f, f.area()))
            .unwrap();
        let screen: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(screen.contains("8 Status"));
        assert!(screen.contains("1 Logs"));
        assert!(screen.contains("z f File"));
        assert!(!screen.contains("1 Status"));
        assert!(!screen.contains("6 Settings"));
    }
}
