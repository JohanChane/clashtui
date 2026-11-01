use super::actions::GlobalAction;
use super::*;
use crate::tui::key::{self, Keymaps, Match, Matcher, Scope};

#[test]
fn default_global_chords_execute_and_clear_pending_state() {
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
        (
            "f",
            GlobalAction::RestartCoreService,
            "Start/restart core service",
        ),
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
            Match::Execute(key::Action::App(AppAction::Global(action))) if action == expected)
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
        Match::Execute(key::Action::App(AppAction::Quit))
    ));
    assert!(matches!(
        input.feed(&"?".parse().unwrap(), Scope::App, Scope::App),
        Match::Execute(key::Action::App(AppAction::Help))
    ));
}

fn global_maps(value: serde_yml::Value) -> anyhow::Result<Keymaps> {
    let bindings = km::bindings(value)?;
    Keymaps::from_bindings([(Scope::App, bindings)].into())
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

fn resolve_global(map: &Keymaps, keys: &[&str]) -> Option<AppAction> {
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
            Match::Execute(key::Action::App(AppAction::TabPrev))
        ));
    }
    assert!(matches!(
        resolve_global(&map, &["<Tab>"]),
        Some(AppAction::TabNext)
    ));
    assert!(docs.contains(&("<S-Tab>".into(), "Tab Previous".into())));

    let (map, docs) = compile_globals(
        "- {on: '<S-Tab>', action: null}\n- {on: [g, p], action: TabPrev, desc: Previous page}",
    )
    .unwrap();
    assert!(resolve_global(&map, &["<S-Tab>"]).is_none());
    assert!(matches!(
        resolve_global(&map, &["g", "p"]),
        Some(AppAction::TabPrev)
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
        Some(AppAction::Global(GlobalAction::RestartCoreService))
    ));
    assert!(matches!(
        resolve_global(&map, &["<C-g>", "c"]),
        Some(AppAction::Global(GlobalAction::CloseAllConnections))
    ));
    assert!(matches!(
        resolve_global(&map, &["<C-g>", "m"]),
        Some(AppAction::Global(GlobalAction::OpenCoreInstallDir))
    ));
    assert!(resolve_global(&map, &["<C-g>", "f"]).is_none());
    assert!(resolve_global(&map, &["q"]).is_none());
    assert!(resolve_global(&map, &["1"]).is_none());
    assert!(matches!(
        resolve_global(&map, &["Q"]),
        Some(AppAction::Quit)
    ));
    assert!(matches!(
        resolve_global(&map, &["<F1>"]),
        Some(AppAction::Help)
    ));
    assert!(matches!(
        resolve_global(&map, &["x"]),
        Some(AppAction::Tab(TabTarget::Status))
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
        ("Quit", AppAction::Quit),
        ("Help", AppAction::Help),
        ("TabNext", AppAction::TabNext),
        ("TabPrev", AppAction::TabPrev),
        ("TabStatus", AppAction::Tab(TabTarget::Status)),
        ("TabFile", AppAction::Tab(TabTarget::File)),
        ("TabProxies", AppAction::Tab(TabTarget::Proxies)),
        ("TabConnections", AppAction::Tab(TabTarget::Connections)),
        ("TabLogs", AppAction::Tab(TabTarget::Logs)),
        ("TabSettings", AppAction::Tab(TabTarget::Settings)),
        ("TabCoreSrvCtl", AppAction::Tab(TabTarget::CoreSrvCtl)),
        (
            "OpenCoreDataDir",
            AppAction::Global(GlobalAction::OpenCoreDataDir),
        ),
        (
            "OpenCoreInstallDir",
            AppAction::Global(GlobalAction::OpenCoreInstallDir),
        ),
        (
            "RestartCoreService",
            AppAction::Global(GlobalAction::RestartCoreService),
        ),
        (
            "CloseAllConnections",
            AppAction::Global(GlobalAction::CloseAllConnections),
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
    let map = global_maps(scopes.remove(&Scope::App).unwrap()).unwrap();
    assert!(resolve_global(&map, &["x"]).is_none());
    assert!(matches!(
        resolve_global(&map, &["y"]),
        Some(AppAction::Quit)
    ));
    assert!(matches!(
        resolve_global(&map, &["q"]),
        Some(AppAction::Quit)
    ));
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
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 3)).unwrap();
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
