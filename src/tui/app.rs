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

#[derive_aliases::derive(..Key, Debug)]
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

impl TabTarget {
    pub(super) const ALL: [Self; 7] = [
        Self::Status,
        Self::File,
        Self::Proxies,
        Self::Connections,
        Self::Logs,
        Self::Settings,
        Self::CoreSrvCtl,
    ];
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

    key_map!(App, AppKey, super::defaults());

    pub(super) fn defaults() -> DefaultMap<AppKey> {
        DefaultMap::new()
            .with_common(
                TabTarget::ALL
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
    input: key::Matcher<'static>,

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
            input: key::Matcher::new(key::keymaps()),
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

    /// Popups capture text input; all page/global shortcuts share one compiled map.
    /// Help can start a global-only chord, which retains its candidates until completion.
    fn handle_key_event(&mut self, kv: &Key) {
        log::debug!("K: {kv:?}");
        if self.popup.check() {
            self.cancel_pending();
            self.popup.handle_key_event(kv);
            return;
        }
        let scope = self.tabs[self.tab_index as usize].input_scope();
        let map_scope = if self.help.is_active() {
            self.cancel_pending();
            if !self.input.starts_chord(key::Scope::App, kv) {
                self.help.handle_key_event(kv);
                return;
            }
            self.help.dismiss();
            key::Scope::App
        } else {
            scope
        };
        if let key::Match::Execute(action) = self.input.feed(kv, scope, map_scope) {
            match action {
                key::Action::App(action) => self.handle_action(action),
                action => self.tabs[self.tab_index as usize].handle_action(action),
            }
        }
    }
    fn render(&mut self, f: &mut ratatui::Frame) {
        use ratatui::prelude::{Constraint, Layout};

        let chunks = Layout::default()
            .constraints([Constraint::Length(3), Constraint::Fill(1)])
            .split(f.area());

        let scope = self.tabs[self.tab_index as usize].input_scope();
        let maps = key::keymaps();
        let hints = self.input.hints(scope);
        render_tabbar(
            tab_titles(self.tabs.iter().map(|tab| tab.title()), maps),
            self.tab_index,
            f,
            chunks[0],
        );

        self.tabs[self.tab_index as usize].render(f, chunks[1]);

        if let Some(hints) = hints {
            widget::which::render(f, &hints);
        }
        if self.help.is_active() {
            self.help
                .render(f, self.tabs[self.tab_index as usize].title(), maps, scope);
        }

        if self.popup.check() {
            self.popup.render(f, Default::default());
        }
    }

    fn cancel_pending(&mut self) {
        self.input.cancel();
    }

    fn switch_tab(&mut self, new_index: u8) {
        if new_index != self.tab_index {
            self.cancel_pending();
            self.tabs[self.tab_index as usize].on_leave();
            self.tab_index = new_index;
            self.tabs[self.tab_index as usize].on_enter();
        }
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

fn tab_titles(titles: impl IntoIterator<Item = &'static str>, maps: &key::Keymaps) -> Vec<String> {
    TabTarget::ALL
        .into_iter()
        .zip(titles)
        .map(|(target, title)| {
            let keys: Vec<_> = maps
                .tab_bindings(target)
                .iter()
                .map(|binding| binding.key_text.as_str())
                .collect();
            if keys.is_empty() {
                title.to_owned()
            } else {
                format!("{} {title}", keys.join("/"))
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
    use crate::tui::key::{self, Keymaps, Match, Matcher, Scope};

    #[test]
    fn global_chords_match_dev_bindings_and_clear_pending_state() {
        let (map, docs) = compile_globals("[]").unwrap();
        let mut input = Matcher::new(&map);
        let prefix = "<C-g>".parse().unwrap();
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
            assert!(matches!(
                input.feed(&prefix, Scope::App, Scope::App),
                Match::Pending
            ));
            let hints = input.hints(Scope::App).unwrap();
            assert_eq!(hints.prefix, prefix);
            assert_eq!(hints.bindings().len(), 4);
            assert!(
                hints
                    .rows()
                    .any(|(candidate, desc)| candidate == key && desc == description)
            );
            assert!(
                matches!(input.feed(&key.parse().unwrap(), Scope::App, Scope::App),
                Match::Execute(key::Action::App(AppKey::Global(action))) if action == expected)
            );
            assert!(input.hints(Scope::App).is_none());
            assert!(docs.contains(&(format!("<C-g> {key}"), description.into())));
        }
        for key in ["<Esc>", "q", "?", "<Tab>", "1", "x"] {
            input.feed(&prefix, Scope::App, Scope::App);
            assert!(matches!(
                input.feed(&key.parse().unwrap(), Scope::App, Scope::App),
                Match::Cancelled
            ));
            assert!(input.hints(Scope::App).is_none());
        }
        input.feed(&prefix, Scope::App, Scope::App);
        input.cancel();
        assert!(input.hints(Scope::App).is_none());
        assert!(matches!(
            input.feed(&"q".parse().unwrap(), Scope::App, Scope::App),
            Match::Execute(key::Action::App(AppKey::Quit))
        ));
        assert!(matches!(
            input.feed(&"?".parse().unwrap(), Scope::App, Scope::App),
            Match::Execute(key::Action::App(AppKey::Help))
        ));
    }

    fn global_maps(value: serde_yml::Value) -> anyhow::Result<Keymaps> {
        let bindings = key::config::resolve(km::defaults(), value)?;
        Keymaps::from_bindings(
            [(
                Scope::App,
                bindings
                    .into_iter()
                    .map(|b| b.map_action(key::Action::App))
                    .collect(),
            )]
            .into(),
        )
    }

    fn compile_globals(yaml: &str) -> anyhow::Result<(Keymaps, Vec<(String, String)>)> {
        let maps = global_maps(serde_yml::from_str(yaml)?)?;
        let docs = maps
            .global_bindings()
            .iter()
            .map(|b| (b.key_text.clone(), b.desc.clone()))
            .collect();
        Ok((maps, docs))
    }

    fn resolve_global(map: &Keymaps, keys: &[&str]) -> Option<AppKey> {
        let mut input = Matcher::new(map);
        let mut action = None;
        for text in keys {
            action = match input.feed(&text.parse().unwrap(), Scope::App, Scope::App) {
                Match::Execute(key::Action::App(action)) => Some(action),
                _ => None,
            };
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
                Matcher::new(&map).feed(&event.into(), Scope::App, Scope::App),
                Match::Execute(key::Action::App(AppKey::TabPrev))
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
        let hints = map
            .chord_view(Scope::App, "<C-g>".parse().unwrap())
            .unwrap();
        assert!(
            hints
                .rows()
                .any(|(key, desc)| key == "r" && desc == "重启核心")
        );
        assert!(
            hints
                .rows()
                .any(|(key, desc)| key == "c" && desc == "Close instead")
        );
        assert!(!hints.rows().any(|(key, _)| key == "f"));
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
        let map = global_maps(scopes.remove("app").unwrap()).unwrap();
        assert!(resolve_global(&map, &["x"]).is_none());
        assert!(matches!(resolve_global(&map, &["y"]), Some(AppKey::Quit)));
        assert!(matches!(resolve_global(&map, &["q"]), Some(AppKey::Quit)));
    }

    #[test]
    fn tabbar_uses_rebound_keys_and_omits_unbound_shortcuts() {
        let app: serde_yml::Value = serde_yml::from_str(
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
        let mut config = serde_yml::Mapping::new();
        config.insert("app".into(), app);
        let maps = key::Keymaps::from_config(config, "mihomo").unwrap();
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
            &maps,
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
