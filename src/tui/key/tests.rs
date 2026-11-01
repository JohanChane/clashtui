use super::*;
use crossterm::event::KeyEvent;
use serde_yml::Value;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
enum Action {
    MoveUp,
    MoveDown,
    GoTop,
    Clear,
}

impl Document for Action {
    fn get_doc(&self) -> &'static str {
        match self {
            Self::MoveUp => "Move up",
            Self::MoveDown => "Move down",
            Self::GoTop => "Go to top",
            Self::Clear => "Clear logs",
        }
    }
}

fn defaults() -> DefaultBindings<Action> {
    DefaultBindings::new()
        .with_keys([
            (KeyCode::Char('j'), Action::MoveDown),
            (KeyCode::Char('k'), Action::MoveUp),
        ])
        .with_chords(KeyCode::Char('g'), [(KeyCode::Char('g'), Action::GoTop)])
}

fn runtime_action(action: Action) -> super::Action {
    use crate::tui::tab::logs;
    super::Action::Logs(match action {
        Action::MoveUp => logs::Action::MoveUp,
        Action::MoveDown => logs::Action::MoveDown,
        Action::GoTop => logs::Action::GoTop,
        Action::Clear => logs::Action::Clear,
    })
}

fn runtime_maps(bindings: Vec<config::ResolvedBinding<Action>>) -> anyhow::Result<Keymaps> {
    Keymaps::from_bindings(
        [(
            Scope::Logs,
            bindings
                .into_iter()
                .map(|b| b.map_action(runtime_action))
                .collect(),
        )]
        .into(),
    )
}

fn docs(view: BindingView<'_>) -> Vec<(String, String)> {
    view.iter()
        .map(|b| (b.key_text.clone(), b.desc.clone()))
        .collect()
}

fn hint_rows(view: &ChordView<'_>) -> Vec<(String, String)> {
    view.rows()
        .map(|(key, desc)| (key, desc.to_owned()))
        .collect()
}

fn compile(yaml: &str) -> anyhow::Result<(Keymaps, Vec<(String, String)>)> {
    let maps = runtime_maps(config::resolve(defaults(), serde_yml::from_str(yaml)?)?)?;
    let descriptions = docs(maps.page_bindings(Scope::Logs));
    Ok((maps, descriptions))
}

fn resolve(map: &Keymaps, keys: &[&str]) -> Option<Action> {
    let mut input = Matcher::new(map);
    let mut result = None;
    for key in keys {
        result = match input.feed(&key.parse().unwrap(), Scope::Logs, Scope::Logs) {
            Match::Execute(action) => [
                Action::MoveUp,
                Action::MoveDown,
                Action::GoTop,
                Action::Clear,
            ]
            .into_iter()
            .find(|candidate| runtime_action(*candidate) == action),
            _ => None,
        };
    }
    assert!(
        input.hints(Scope::Logs).is_none(),
        "test sequence must finish or cancel the chord"
    );
    result
}

fn scope_docs(scope: Scope, overrides: Value) -> anyhow::Result<Vec<(String, String)>> {
    let (name, overrides) = match scope {
        Scope::Profile => (
            "file",
            Value::Mapping([("profile".into(), overrides)].into_iter().collect()),
        ),
        Scope::Template => (
            "file",
            Value::Mapping([("template".into(), overrides)].into_iter().collect()),
        ),
        _ => (scope.name(), overrides),
    };
    let maps = Keymaps::from_config([(name.into(), overrides)].into_iter().collect(), "mihomo")?;
    Ok(docs(maps.page_bindings(scope)))
}

mod configuration {
    use super::*;

    #[test]
    fn partial_overrides_keep_other_defaults_and_replace_exact_sequences() {
        let (map, docs) = compile("- on: j\n  action: Clear\n  desc: 清空日志").unwrap();
        assert_eq!(resolve(&map, &["j"]), Some(Action::Clear));
        assert_eq!(resolve(&map, &["k"]), Some(Action::MoveUp));
        assert_eq!(resolve(&map, &["g", "g"]), Some(Action::GoTop));
        assert!(docs.contains(&("j".into(), "清空日志".into())));
        assert!(docs.contains(&("k".into(), "Move up".into())));
    }

    #[test]
    fn replacing_a_chord_changes_action_and_description() {
        let (map, docs) = compile("- on: [g, g]\n  action: Clear\n  desc: Custom chord").unwrap();
        assert_eq!(resolve(&map, &["g", "g"]), Some(Action::Clear));
        assert_eq!(docs.iter().filter(|(key, _)| key == "g g").count(), 1);
        assert!(docs.contains(&("g g".into(), "Custom chord".into())));
    }

    #[test]
    fn duplicate_and_prefix_conflicts_are_rejected_before_building_the_map() {
        for yaml in [
            "- {on: x, action: Clear}\n- {on: x, action: MoveUp}",
            "- {on: [x, x], action: Clear}\n- {on: [x, x], action: MoveUp}",
            "- {on: G, action: Clear}\n- {on: '<S-g>', action: MoveUp}",
        ] {
            let err = compile(yaml).err().unwrap();
            assert!(format!("{err:#}").contains("duplicate key binding"));
        }
        for yaml in [
            "- {on: g, action: Clear}",
            "- {on: [j, j], action: Clear}",
            "- {on: x, action: Clear}\n- {on: [x, x], action: MoveUp}",
        ] {
            let err = compile(yaml).err().unwrap();
            assert!(format!("{err:#}").contains("prefix conflict"));
        }
    }

    #[test]
    fn bad_configurations_return_errors_instead_of_panicking() {
        for yaml in [
            "- {on: [], action: Clear}",
            "- {on: [g, g, g], action: Clear}",
            "- {on: '', action: Clear}",
            "- {on: gg, action: Clear}",
            "- {on: '<Unknown>', action: Clear}",
            "- {on: x, action: UnknownAction}",
            "- {on: x, action: 'MoveDown 30%'}",
            "- {on: x, exec: Clear}",
            "- {on: x}",
            "- {on: x, action: Clear, typo: true}",
            "- {on: [x, 3], action: Clear}",
            "true",
        ] {
            assert!(
                compile(yaml).is_err(),
                "accepted invalid configuration: {yaml}"
            );
        }
    }

    #[test]
    fn legacy_configuration_formats_are_rejected() {
        for yaml in [
            "- {on: x, action: {Action: Clear}}",
            "- on: x\n  action: !Action Clear",
            "- {on: x, action: [Clear]}",
            "- {on: x, action: true}",
            "- {on: x, action: 42}",
            "- on: x\n  action: !Legacy null",
            "- on: x\n  action: !Legacy Clear",
            "common: {}\nsubmap: {}",
            "x: Clear",
            "x: {action: Clear, desc: Custom}",
            "? code: !Char x\n  ctrl: true\n: action: Clear\n  desc: Custom control binding",
        ] {
            assert!(
                compile(yaml).is_err(),
                "accepted legacy configuration: {yaml}"
            );
        }
    }

    #[test]
    fn core_specific_file_scopes_override_only_the_selected_core() {
        let yaml = "logs: [{on: x, action: Clear}]\nfile:\n  profile: [{on: e, action: Edit}]\nmihomo:\n  logs: [{on: y, action: Clear}]\n  file:\n    template: [{on: p, action: Preview}]\nsing-box:\n  logs: [{on: z, action: Clear}]";
        let input = serde_yml::from_str(yaml).unwrap();
        let map = config::sections(input, "mihomo").unwrap();
        assert!(map.contains_key(&Scope::Profile));
        assert!(map.contains_key(&Scope::Template));
        let compiled =
            runtime_maps(config::resolve(defaults(), map[&Scope::Logs].clone()).unwrap()).unwrap();
        assert_eq!(resolve(&compiled, &["y"]), Some(Action::Clear));
        assert_eq!(resolve(&compiled, &["x"]), None);
        assert_eq!(resolve(&compiled, &["z"]), None);
    }

    #[test]
    fn malformed_scopes_are_rejected() {
        for yaml in [
            "log: []",
            "file: []",
            "file: {unknown: []}",
            "mihomo: []",
            "file: {profile: []}\nfiles/profile: []",
            "files/profile: []",
        ] {
            assert!(config::sections(serde_yml::from_str(yaml).unwrap(), "mihomo").is_err());
        }
    }

    #[test]
    fn missing_and_empty_keymap_files_use_defaults_but_invalid_yaml_does_not() {
        let path = std::env::temp_dir().join(format!("clashtui-keymap-{}.yaml", fastrand::u64(..)));
        assert!(read_keymap(&path).unwrap().is_empty());
        assert!(!path.exists());
        std::fs::write(&path, "{}\n").unwrap();
        assert!(read_keymap(&path).unwrap().is_empty());
        std::fs::write(&path, "# User overrides\n").unwrap();
        assert!(read_keymap(&path).unwrap().is_empty());
        std::fs::write(&path, "logs: [").unwrap();
        assert!(read_keymap(&path).is_err());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn unbinding_allows_single_keys_and_chords_to_replace_each_other() {
        let (map, docs) =
            compile("- {on: j, action: null}\n- {on: [j, j], action: Clear}").unwrap();
        assert_eq!(resolve(&map, &["j", "j"]), Some(Action::Clear));
        assert!(!docs.iter().any(|(key, _)| key == "j"));
        let (map, _) = compile("- {on: [g, g], action: null}\n- {on: g, action: Clear}").unwrap();
        assert_eq!(resolve(&map, &["g"]), Some(Action::Clear));
        let (map, _) = compile("- {on: j, action: null}").unwrap();
        assert_eq!(resolve(&map, &["j"]), None);
        assert_eq!(resolve(&map, &["k"]), Some(Action::MoveUp));
    }

    #[test]
    fn file_actions_use_plain_names_and_reject_nested_actions() {
        for (name, description) in [
            ("Add", "Import (URL or file)"),
            ("Delete", "Delete"),
            ("Edit", "Edit"),
            ("Preview", "Preview"),
            ("Update", "Update"),
            ("UpdateAll", "Update all"),
            ("Search", "Search/Filter"),
            ("Test", "Test"),
            ("Check", "Check config"),
            ("FzfFind", "Find profile"),
            ("ToggleNoPp", "Toggle no proxy-provider"),
            ("ToggleUpdateWithProxy", "Toggle update with proxy"),
            ("Traffic", "Show traffic"),
            ("GoTop", "Go to top"),
            ("GoEnd", "Go to bottom"),
        ] {
            let input = serde_yml::from_str(&format!("- {{on: x, action: {name}}}")).unwrap();
            let docs = scope_docs(Scope::Profile, input).unwrap();
            assert!(docs.contains(&("x".into(), description.into())), "{name}");
        }
        for (name, description) in [
            ("Generate", "Generate"),
            ("Delete", "Delete"),
            ("Edit", "Edit"),
            ("EditProviders", "Edit proxy providers"),
            ("Preview", "Preview"),
            ("Search", "Search/Filter"),
            ("FzfFind", "Find template"),
        ] {
            let input = serde_yml::from_str(&format!("- {{on: x, action: {name}}}")).unwrap();
            let docs = scope_docs(Scope::Template, input).unwrap();
            assert!(docs.contains(&("x".into(), description.into())), "{name}");
        }
        for yaml in [
            "- {on: x, action: {Action: Edit}}",
            "- on: x\n  action: !Action Edit",
            "- {on: x, action: UnknownAction}",
        ] {
            let input: Value = serde_yml::from_str(yaml).unwrap();
            assert!(scope_docs(Scope::Profile, input.clone()).is_err());
            assert!(scope_docs(Scope::Template, input).is_err());
        }
        assert!(
            scope_docs(
                Scope::Profile,
                serde_yml::from_str("- {on: x, action: Generate}").unwrap()
            )
            .is_err()
        );
        assert!(
            scope_docs(
                Scope::Template,
                serde_yml::from_str("- {on: x, action: Update}").unwrap()
            )
            .is_err()
        );
    }

    #[test]
    fn sorting_uses_public_names_without_nested_configuration() {
        for (name, description) in [
            ("SortByName", "Sort by name"),
            ("SortByDelay", "Sort by delay"),
            ("ResetSort", "Reset sort"),
            ("GlobalSortByName", "Global sort by name"),
            ("GlobalSortByDelay", "Global sort by delay"),
            ("GlobalResetSort", "Global reset sort"),
        ] {
            let input = serde_yml::from_str(&format!("- {{on: x, action: {name}}}")).unwrap();
            let docs = scope_docs(Scope::Proxies, input).unwrap();
            assert!(docs.contains(&("x".into(), description.into())), "{name}");
        }
        for name in [
            "Host", "Rule", "Chains", "Download", "Upload", "DlSpeed", "UlSpeed",
        ] {
            let input = serde_yml::from_str(&format!("- {{on: x, action: SortBy{name}}}")).unwrap();
            scope_docs(Scope::Connections, input).unwrap();
        }
        for yaml in [
            "- {on: x, action: {Sort: ByDelay}}",
            "- on: x\n  action: !Sort ByDelay",
            "- {on: x, action: ByDelay}",
        ] {
            assert!(scope_docs(Scope::Proxies, serde_yml::from_str(yaml).unwrap()).is_err());
        }
        for yaml in [
            "- {on: x, action: {SortBy: Host}}",
            "- on: x\n  action: !SortBy Host",
            "- {on: x, action: Host}",
        ] {
            assert!(scope_docs(Scope::Connections, serde_yml::from_str(yaml).unwrap()).is_err());
        }
    }
}

mod matching {
    use super::*;

    #[test]
    fn chords_and_modified_keys_dispatch_with_updated_descriptions() {
        let (map, docs) = compile("- on: [g, e]\n  action: Clear\n  desc: Clear by chord\n- on: [\"<C-g>\", \"<A-x>\"]\n  action: MoveDown\n- on: \"<C-j>\"\n  action: MoveDown").unwrap();
        assert_eq!(resolve(&map, &["g", "e"]), Some(Action::Clear));
        assert_eq!(resolve(&map, &["g", "g"]), Some(Action::GoTop));
        assert_eq!(resolve(&map, &["<C-g>", "<A-x>"]), Some(Action::MoveDown));
        assert_eq!(resolve(&map, &["<C-j>"]), Some(Action::MoveDown));
        assert_eq!(resolve(&map, &["j"]), Some(Action::MoveDown));
        assert!(docs.contains(&("g e".into(), "Clear by chord".into())));
    }

    #[test]
    fn unmatched_second_key_clears_pending_state() {
        let (map, _) = compile("[]").unwrap();
        assert_eq!(resolve(&map, &["g", "x"]), None);
        assert_eq!(resolve(&map, &["j"]), Some(Action::MoveDown));
    }

    #[test]
    fn ascii_letters_and_backtab_keep_their_normalized_forms() {
        let uppercase = "G".parse::<Key>().unwrap();
        for (c, modifiers) in [
            ('G', KeyModifiers::NONE),
            ('G', KeyModifiers::SHIFT),
            ('g', KeyModifiers::SHIFT),
        ] {
            assert_eq!(
                Key::from(KeyEvent::new(KeyCode::Char(c), modifiers)),
                uppercase
            );
        }
        assert_eq!("<S-g>".parse::<Key>().unwrap(), uppercase);
        assert_eq!(uppercase.to_string(), "G");
        assert_eq!(
            Key::from(KeyEvent::new(
                KeyCode::BackTab,
                KeyModifiers::CONTROL | KeyModifiers::SHIFT
            )),
            "<C-S-Tab>".parse().unwrap()
        );
    }

    #[test]
    fn shifted_characters_do_not_replace_or_unbind_plain_defaults() {
        for (c, plain, shifted) in [
            ('1', "1", "<S-1>"),
            ('/', "/", "<S-/>"),
            ('!', "!", "<S-!>"),
            ('?', "?", "<S-?>"),
            (' ', "<Space>", "<S-Space>"),
            ('é', "é", "<S-é>"),
        ] {
            for action in ["Clear", "null"] {
                let defaults =
                    DefaultBindings::new().with_keys([(KeyCode::Char(c), Action::MoveDown)]);
                let config =
                    serde_yml::from_str(&format!("- {{on: '{shifted}', action: {action}}}"))
                        .unwrap();
                let map = runtime_maps(config::resolve(defaults, config).unwrap()).unwrap();
                let mut input = Matcher::new(&map);
                let plain_event = KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
                let shifted_event = KeyEvent::new(KeyCode::Char(c), KeyModifiers::SHIFT);
                assert_eq!(
                    input.feed(&plain_event.into(), Scope::Logs, Scope::Logs),
                    Match::Execute(runtime_action(Action::MoveDown)),
                    "{shifted}: {action} must not change {plain}"
                );
                let expected = if action == "Clear" {
                    Match::Execute(runtime_action(Action::Clear))
                } else {
                    Match::Unmatched
                };
                assert_eq!(
                    input.feed(&shifted_event.into(), Scope::Logs, Scope::Logs),
                    expected,
                    "{shifted}: {action}"
                );
                let descriptions = docs(map.page_bindings(Scope::Logs));
                assert!(descriptions.contains(&(plain.into(), "Move down".into())));
                if action == "Clear" {
                    assert!(descriptions.contains(&(shifted.into(), "Clear logs".into())));
                }
            }
        }
    }

    #[test]
    fn key_notation_round_trips() {
        for text in [
            "j",
            "G",
            "S",
            "C",
            "A",
            "D",
            "1",
            "!",
            "-",
            "é",
            "?",
            "中",
            " ",
            "<",
            ">",
            "<C-u>",
            "<A-x>",
            "<D-x>",
            "<C-S-a>",
            "<S-g>",
            "<S-s>",
            "<S-1>",
            "<S-/>",
            "<S-!>",
            "<S-?>",
            "<S-Space>",
            "<S-é>",
            "<C-S-1>",
            "<A-S-/>",
            "<D-S-Space>",
            "<S-Up>",
            "<BackTab>",
            "<C-->",
            "<C-<>",
            "<C->>",
            "<Space>",
            "<F12>",
            "<PageDown>",
        ] {
            let key: Key = text.parse().unwrap();
            assert_eq!(key.to_string().parse::<Key>().unwrap(), key, "{text}");
        }
        for text in [
            "",
            "gg",
            "<C->",
            "<F0>",
            "<F25>",
            "<NoSuchKey>",
            "<C-j",
            "C-j>",
        ] {
            assert!(text.parse::<Key>().is_err(), "accepted {text}");
        }
    }

    #[test]
    fn bare_characters_in_angle_brackets_report_how_to_write_them() {
        for character in [
            'S', 'C', 'A', 'D', 'g', '1', '!', '-', '<', '>', ' ', 'é', '中',
        ] {
            let text = format!("<{character}>");
            let error = text.parse::<Key>().unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains(&format!("write `{character}` without angle brackets")),
                "{text}: {error}"
            );
        }
    }

    #[test]
    fn pending_hints_follow_overrides_unbinding_and_stable_key_order() {
        let (map, _) = compile("- {on: [g, z], action: Clear, desc: Last}\n- {on: [g, a], action: MoveUp, desc: First}\n- {on: [g, g], action: null}").unwrap();
        let mut input = Matcher::new(&map);
        assert_eq!(
            input.feed(&"g".parse().unwrap(), Scope::Logs, Scope::Logs),
            Match::Pending
        );
        let hints = input.hints(Scope::Logs).unwrap();
        assert_eq!(hints.prefix, "g".parse().unwrap());
        assert_eq!(
            hint_rows(&hints),
            vec![("a".into(), "First".into()), ("z".into(), "Last".into())]
        );
        assert_eq!(
            input.feed(&"a".parse().unwrap(), Scope::Logs, Scope::Logs),
            Match::Execute(runtime_action(Action::MoveUp))
        );
        assert!(input.hints(Scope::Logs).is_none());
    }

    #[test]
    fn escape_cancels_pending_input_and_is_reserved_as_a_second_key() {
        let (map, _) =
            compile("- {on: '<Esc>', action: Clear}\n- {on: [g, '<C-Esc>'], action: MoveDown}")
                .unwrap();
        let mut input = Matcher::new(&map);
        input.feed(&"g".parse().unwrap(), Scope::Logs, Scope::Logs);
        assert_eq!(
            input.feed(&"<Esc>".parse().unwrap(), Scope::Logs, Scope::Logs),
            Match::Cancelled
        );
        assert!(input.hints(Scope::Logs).is_none());
        assert_eq!(resolve(&map, &["<Esc>"]), Some(Action::Clear));
        assert_eq!(resolve(&map, &["g", "<C-Esc>"]), Some(Action::MoveDown));
        let err = compile("- {on: [g, '<Esc>'], action: Clear}")
            .err()
            .unwrap();
        assert!(format!("{err:#}").contains("Esc is reserved"));
    }
}

mod windows_input {
    use super::*;
    use crate::tui::{app::AppAction, tab::logs};
    use matcher::InputPolicy;

    fn maps(yaml: &str) -> Keymaps {
        Keymaps::from_config(serde_yml::from_str(yaml).unwrap(), "mihomo").unwrap()
    }

    fn press(input: &mut Matcher<'_>, key: &str) -> Match {
        input.feed(&key.parse().unwrap(), Scope::Logs, Scope::Logs)
    }

    #[test]
    fn generated_characters_fall_back_only_for_native_windows_input() {
        for c in ['?', '!', '1', '/', 'é', '中'] {
            let defaults = DefaultBindings::new().with_keys([(KeyCode::Char(c), Action::Clear)]);
            let maps = runtime_maps(config::resolve(defaults, Value::Null).unwrap()).unwrap();
            let event = KeyEvent::new(KeyCode::Char(c), KeyModifiers::SHIFT);
            let key = Key::from(event);
            for policy in [InputPolicy::Exact, InputPolicy::WindowsNative] {
                let mut input = Matcher::with_policy(&maps, policy);
                let expected = match policy {
                    InputPolicy::Exact => Match::Unmatched,
                    InputPolicy::WindowsNative => Match::Execute(runtime_action(Action::Clear)),
                };
                assert_eq!(input.feed(&key, Scope::Logs, Scope::Logs), expected, "{c}");
                assert_eq!(key, Key::from(event), "fallback must not rewrite the event");
                assert_eq!(
                    input.feed(&Key::from_code(KeyCode::Char(c)), Scope::Logs, Scope::Logs),
                    Match::Execute(runtime_action(Action::Clear))
                );
            }
        }
    }

    #[test]
    fn whitespace_controls_named_keys_and_modified_combinations_stay_exact() {
        for code in [
            KeyCode::Char(' '),
            KeyCode::Char('\u{a0}'),
            KeyCode::Char('\u{2003}'),
            KeyCode::Char('\0'),
            KeyCode::Char('\n'),
            KeyCode::Char('\u{85}'),
            KeyCode::Enter,
            KeyCode::Tab,
            KeyCode::F(1),
            KeyCode::Left,
            // Existing ASCII letter normalization must not turn Shift+a into a.
            KeyCode::Char('a'),
        ] {
            let defaults = DefaultBindings::new().with_keys([(code, Action::Clear)]);
            let maps = runtime_maps(config::resolve(defaults, Value::Null).unwrap()).unwrap();
            let mut input = Matcher::with_policy(&maps, InputPolicy::WindowsNative);
            assert_eq!(
                input.feed(
                    &KeyEvent::new(code, KeyModifiers::SHIFT).into(),
                    Scope::Logs,
                    Scope::Logs
                ),
                Match::Unmatched,
                "{code:?}"
            );
        }
        for extra in [
            KeyModifiers::CONTROL,
            KeyModifiers::ALT,
            KeyModifiers::SUPER,
        ] {
            let base = Key::from(KeyEvent::new(KeyCode::Char('?'), extra));
            let defaults = DefaultBindings::new().with_keys([(base, Action::Clear)]);
            let maps = runtime_maps(config::resolve(defaults, Value::Null).unwrap()).unwrap();
            let mut input = Matcher::with_policy(&maps, InputPolicy::WindowsNative);
            assert_eq!(
                input.feed(
                    &KeyEvent::new(KeyCode::Char('?'), extra | KeyModifiers::SHIFT).into(),
                    Scope::Logs,
                    Scope::Logs
                ),
                Match::Unmatched
            );
            assert_eq!(
                input.feed(&base, Scope::Logs, Scope::Logs),
                Match::Execute(runtime_action(Action::Clear))
            );
        }
    }

    #[test]
    fn default_help_accepts_windows_question_mark_without_a_second_binding() {
        let maps = maps("{}");
        let mut input = Matcher::with_policy(&maps, InputPolicy::WindowsNative);
        for scope in Scope::ALL {
            let key = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::SHIFT).into();
            assert_eq!(
                input.feed(&key, scope, scope),
                Match::Execute(super::super::Action::App(AppAction::Help))
            );
        }
        assert_eq!(
            maps.global_bindings()
                .iter()
                .filter(|b| b.action == super::super::Action::App(AppAction::Help))
                .map(|b| b.key_text.as_str())
                .collect::<Vec<_>>(),
            vec!["?"]
        );
        // A layout reporting '/' with Shift produces '/', never a guessed '?'.
        assert_eq!(
            press(&mut input, "<S-/>"),
            Match::Execute(super::super::Action::Logs(logs::Action::Search))
        );
        let expected = if cfg!(windows) {
            Match::Execute(super::super::Action::App(AppAction::Help))
        } else {
            Match::Unmatched
        };
        assert_eq!(press(&mut Matcher::new(&maps), "<S-?>"), expected);
    }

    #[test]
    fn exact_bindings_win_across_the_whole_effective_context() {
        for (yaml, expected) in [
            (
                "app: [{on: '!', action: TabPrev}]\nlogs: [{on: '<S-!>', action: Clear}]",
                super::super::Action::Logs(logs::Action::Clear),
            ),
            (
                "app: [{on: '<S-!>', action: TabPrev}]\nlogs: [{on: '!', action: Clear}]",
                super::super::Action::App(AppAction::TabPrev),
            ),
            (
                "app: [{on: '<S-!>', action: null}]\nlogs: [{on: '<S-!>', action: Clear}]",
                super::super::Action::Logs(logs::Action::Clear),
            ),
            (
                "app: [{on: '<S-!>', action: TabPrev}]\nlogs: [{on: '<S-!>', action: null}]",
                super::super::Action::App(AppAction::TabPrev),
            ),
        ] {
            let maps = maps(yaml);
            let mut input = Matcher::with_policy(&maps, InputPolicy::WindowsNative);
            assert_eq!(
                press(&mut input, "<S-!>"),
                Match::Execute(expected),
                "{yaml}"
            );
        }
    }

    #[test]
    fn explicit_unbindings_block_fallback_without_becoming_help_rows() {
        for yaml in [
            "app: [{on: '<S-?>', action: null}]",
            "logs: [{on: '<S-?>', action: null}]",
        ] {
            let maps = maps(yaml);
            let mut input = Matcher::with_policy(&maps, InputPolicy::WindowsNative);
            assert_eq!(press(&mut input, "<S-?>"), Match::Unmatched);
            assert_eq!(
                press(&mut input, "?"),
                Match::Execute(super::super::Action::App(AppAction::Help))
            );
            assert!(
                maps.effective_bindings(Scope::Logs)
                    .all(|(_, b)| b.key_text != "<S-?>")
            );
            assert!(input.hints(Scope::Logs).is_none());
            if yaml.starts_with("logs:") {
                assert_eq!(
                    input.feed(&"<S-?>".parse().unwrap(), Scope::App, Scope::App),
                    Match::Execute(super::super::Action::App(AppAction::Help))
                );
            }
        }
    }

    #[test]
    fn exact_single_key_prevents_starting_a_fallback_chord() {
        let maps = maps("app: [{on: ['!', x], action: TabPrev}, {on: '<S-!>', action: Help}]");
        let mut input = Matcher::with_policy(&maps, InputPolicy::WindowsNative);
        let key = "<S-!>".parse().unwrap();
        assert!(!input.starts_chord(Scope::App, &key));
        assert_eq!(
            press(&mut input, "<S-!>"),
            Match::Execute(super::super::Action::App(AppAction::Help))
        );
        assert!(input.hints(Scope::Logs).is_none());
    }

    #[test]
    fn exact_chord_prefix_owns_the_sequence_before_any_fallback() {
        let maps =
            maps("app: [{on: '!', action: TabPrev}]\nlogs: [{on: ['<S-!>', x], action: Clear}]");
        let mut input = Matcher::with_policy(&maps, InputPolicy::WindowsNative);
        for second in ["?", "<S-?>", "!", "<Esc>", "x"] {
            assert_eq!(press(&mut input, "<S-!>"), Match::Pending);
            assert_eq!(
                input.hints(Scope::Logs).unwrap().prefix,
                "<S-!>".parse().unwrap()
            );
            assert_eq!(
                press(&mut input, second),
                if second == "x" {
                    Match::Execute(runtime_action(Action::Clear))
                } else {
                    Match::Cancelled
                }
            );
            assert!(input.hints(Scope::Logs).is_none());
        }
    }

    #[test]
    fn fallback_chords_keep_the_matched_prefix_and_exact_second_key_priority() {
        let maps = maps(
            "logs: [{on: ['!', '?'], action: Clear}, {on: ['!', '<S-?>'], action: MoveUp}, {on: ['!', '/'], action: MoveDown}]",
        );
        let mut input = Matcher::with_policy(&maps, InputPolicy::WindowsNative);
        for (second, action) in [
            ("<S-?>", Action::MoveUp),
            ("?", Action::Clear),
            ("<S-/>", Action::MoveDown),
        ] {
            assert_eq!(press(&mut input, "<S-!>"), Match::Pending);
            let hints = input.hints(Scope::Logs).unwrap();
            assert_eq!(hints.prefix, "!".parse().unwrap());
            assert_eq!(hints.bindings().len(), 3);
            assert_eq!(
                press(&mut input, second),
                Match::Execute(runtime_action(action))
            );
        }
    }

    #[test]
    fn disabled_chords_block_fallback_but_do_not_hide_active_rebindings() {
        let maps = maps(
            "app: [{on: ['!', '<S-?>'], action: null}]\nlogs: [{on: ['!', '?'], action: Clear}, {on: ['!', x], action: MoveUp}, {on: '<S-!>', action: null}]",
        );
        let mut input = Matcher::with_policy(&maps, InputPolicy::WindowsNative);
        assert_eq!(press(&mut input, "<S-!>"), Match::Unmatched);
        for (second, expected) in [
            ("<S-?>", Match::Cancelled),
            ("?", Match::Execute(runtime_action(Action::Clear))),
        ] {
            assert_eq!(press(&mut input, "!"), Match::Pending);
            assert_eq!(input.hints(Scope::Logs).unwrap().bindings().len(), 2);
            assert_eq!(press(&mut input, second), expected);
        }

        let maps =
            self::maps("app: [{on: ['<S-!>', x], action: null}, {on: ['!', x], action: TabPrev}]");
        let mut input = Matcher::with_policy(&maps, InputPolicy::WindowsNative);
        assert_eq!(press(&mut input, "<S-!>"), Match::Unmatched);
        assert!(!input.starts_chord(Scope::Logs, &"<S-!>".parse().unwrap()));
        assert!(input.hints(Scope::Logs).is_none());

        let maps = self::maps(
            "app: [{on: ['!', '<S-?>'], action: null}]\nlogs: [{on: ['!', '<S-?>'], action: Clear}, {on: '!', action: null}]",
        );
        let mut input = Matcher::with_policy(&maps, InputPolicy::WindowsNative);
        assert_eq!(press(&mut input, "<S-!>"), Match::Pending);
        assert_eq!(
            press(&mut input, "<S-?>"),
            Match::Execute(runtime_action(Action::Clear))
        );
    }

    #[test]
    fn chords_started_from_help_use_fallback_without_gaining_page_candidates() {
        let maps = maps(
            "app: [{on: ['!', '?'], action: TabPrev}]\nlogs: [{on: ['!', '/'], action: Clear}]",
        );
        let mut input = Matcher::with_policy(&maps, InputPolicy::WindowsNative);
        let prefix = "<S-!>".parse().unwrap();
        assert!(input.starts_chord(Scope::App, &prefix));
        for second in ["<S-/>", "<S-?>"] {
            assert_eq!(input.feed(&prefix, Scope::Logs, Scope::App), Match::Pending);
            let hints = input.hints(Scope::Logs).unwrap();
            assert_eq!(hints.prefix, "!".parse().unwrap());
            assert_eq!(hints.bindings().len(), 1);
            assert_eq!(
                press(&mut input, second),
                if second == "<S-?>" {
                    Match::Execute(super::super::Action::App(AppAction::TabPrev))
                } else {
                    Match::Cancelled
                }
            );
        }
    }
}

mod contexts {
    use super::{docs, hint_rows};
    use crate::tui::key::*;
    use crate::tui::{
        app::{AppAction, TabTarget},
        tab::{files, logs},
    };

    fn maps(yaml: &str) -> anyhow::Result<Keymaps> {
        Keymaps::from_config(serde_yml::from_str(yaml)?, "mihomo")
    }

    fn press(input: &mut Matcher<'_>, scope: Scope, key: &str) -> Match {
        input.feed(&key.parse().unwrap(), scope, scope)
    }

    #[test]
    fn defaults_and_reference_configuration_have_no_cross_scope_conflicts() {
        for core in ["mihomo", "sing-box"] {
            Keymaps::from_config(serde_yml::Mapping::new(), core).unwrap();
            Keymaps::from_config(
                serde_yml::from_str(include_str!(
                    "../../../contrib/default_configs/default_keymap.yaml"
                ))
                .unwrap(),
                core,
            )
            .unwrap();
        }
    }

    #[test]
    fn global_collisions_report_both_scopes_sequences_and_actions() {
        for (yaml, scope, global_key, page_key) in [
            ("logs: [{on: q, action: Clear}]", "logs", "q", "q"),
            ("logs: [{on: '1', action: Clear}]", "logs", "1", "1"),
            (
                "logs: [{on: ['<C-g>', c], action: Clear}]",
                "logs",
                "<C-g> c",
                "<C-g> c",
            ),
            (
                "app: [{on: [g, g], action: Help}]",
                "connections",
                "g g",
                "g g",
            ),
            ("app: [{on: g, action: Help}]", "connections", "g", "g g"),
            ("logs: [{on: [q, x], action: Clear}]", "logs", "q", "q x"),
            (
                "logs: [{on: '<C-g>', action: Clear}]",
                "logs",
                "<C-g> c",
                "<C-g>",
            ),
            ("app: [{on: [c, x], action: Help}]", "logs", "c x", "c"),
            (
                "app: [{on: '<S-g>', action: Help}]",
                "connections",
                "G",
                "G",
            ),
            (
                "file: {profile: [{on: q, action: Preview}]}",
                "file.profile",
                "q",
                "q",
            ),
            (
                "file: {template: [{on: q, action: Preview}]}",
                "file.template",
                "q",
                "q",
            ),
        ] {
            let error = maps(yaml)
                .err()
                .expect("a conflicting configuration must fail")
                .to_string();
            assert!(error.contains("key binding conflict"), "{error}");
            assert!(
                error.contains(&format!("`app` `{global_key}` (")),
                "{error}"
            );
            assert!(
                error.contains(&format!("`{scope}` `{page_key}` (")),
                "{error}"
            );
            assert!(error.contains("rebind or unbind"), "{error}");
        }
    }

    #[test]
    fn shared_prefix_uses_one_candidate_set_for_matching_help_and_hints() {
        let maps = maps("app: [{on: [g, p], action: TabPrev, desc: Previous page}]\nlogs: [{on: [g, g], action: GoTop, desc: First log}]").unwrap();
        assert!(
            docs(maps.page_bindings(Scope::Logs)).contains(&("g g".into(), "First log".into()))
        );
        assert!(docs(maps.global_bindings()).contains(&("g p".into(), "Previous page".into())));
        let mut input = Matcher::new(&maps);
        for second in ["g", "p"] {
            assert!(matches!(
                press(&mut input, Scope::Logs, "g"),
                Match::Pending
            ));
            assert_eq!(
                hint_rows(&input.hints(Scope::Logs).unwrap()),
                vec![
                    ("g".into(), "First log".into()),
                    ("p".into(), "Previous page".into())
                ]
            );
            let action = press(&mut input, Scope::Logs, second);
            if second == "g" {
                assert!(matches!(
                    action,
                    Match::Execute(Action::Logs(logs::Action::GoTop))
                ));
            } else {
                assert!(matches!(
                    action,
                    Match::Execute(Action::App(AppAction::TabPrev))
                ));
            }
            assert!(input.hints(Scope::Logs).is_none());
        }
    }

    #[test]
    fn unbinding_and_rebinding_remove_conflicts_before_contexts_are_compiled() {
        let maps = maps("app: [{on: q, action: null}, {on: x, action: Quit}, {on: ['<C-g>', c], action: null}]\nlogs: [{on: q, action: Clear}, {on: ['<C-g>', c], action: Clear}]").unwrap();
        let mut input = Matcher::new(&maps);
        assert!(matches!(
            press(&mut input, Scope::Logs, "q"),
            Match::Execute(Action::Logs(logs::Action::Clear))
        ));
        assert!(matches!(
            press(&mut input, Scope::Logs, "x"),
            Match::Execute(Action::App(AppAction::Quit))
        ));
        press(&mut input, Scope::Logs, "<C-g>");
        assert!(matches!(
            press(&mut input, Scope::Logs, "c"),
            Match::Execute(Action::Logs(logs::Action::Clear))
        ));
        assert!(
            !docs(maps.global_bindings())
                .iter()
                .any(|(key, _)| key == "q" || key == "<C-g> c")
        );
    }

    #[test]
    fn only_the_selected_core_participates_in_cross_scope_validation() {
        let yaml = "logs: [{on: q, action: Clear}]\nmihomo:\n  app: [{on: q, action: null}]\nsing-box:\n  app: []";
        assert!(Keymaps::from_config(serde_yml::from_str(yaml).unwrap(), "mihomo").is_ok());
        assert!(Keymaps::from_config(serde_yml::from_str(yaml).unwrap(), "sing-box").is_err());
    }

    #[test]
    fn separate_pages_and_file_panes_can_reuse_the_same_sequence() {
        let maps = maps("logs: [{on: x, action: Clear}]\nfile:\n  profile: [{on: x, action: Preview, desc: Preview profile}]\n  template: [{on: x, action: Preview, desc: Preview template}]").unwrap();
        let mut input = Matcher::new(&maps);
        assert!(matches!(
            press(&mut input, Scope::Logs, "x"),
            Match::Execute(Action::Logs(logs::Action::Clear))
        ));
        assert!(matches!(
            press(&mut input, Scope::Profile, "x"),
            Match::Execute(Action::Profile(files::profile::Action::Operation(
                files::profile::Operation::Preview
            )))
        ));
        assert!(matches!(
            press(&mut input, Scope::Template, "x"),
            Match::Execute(Action::Template(files::template::Action::Operation(
                files::template::Operation::Preview
            )))
        ));
        assert!(
            docs(maps.page_bindings(Scope::Profile))
                .contains(&("x".into(), "Preview profile".into()))
        );
        assert!(
            docs(maps.page_bindings(Scope::Template))
                .contains(&("x".into(), "Preview template".into()))
        );
    }

    #[test]
    fn pending_context_is_cancelled_on_focus_change_or_explicit_cancellation() {
        let maps = maps("file: {template: [{on: x, action: Preview}]}").unwrap();
        let mut input = Matcher::new(&maps);
        press(&mut input, Scope::Profile, "g");
        assert!(input.hints(Scope::Template).is_none());
        assert!(matches!(
            press(&mut input, Scope::Template, "x"),
            Match::Execute(Action::Template(_))
        ));
        press(&mut input, Scope::Template, "g");
        input.cancel();
        assert!(input.hints(Scope::Template).is_none());
        assert!(matches!(
            press(&mut input, Scope::Template, "q"),
            Match::Execute(Action::App(AppAction::Quit))
        ));
    }

    #[test]
    fn cancelled_chords_consume_the_second_key_without_falling_through() {
        let maps = maps("app: [{on: [g, p], action: TabPrev}]").unwrap();
        let mut input = Matcher::new(&maps);
        for second in ["<Esc>", "q", "?", "<Tab>", "1", "<C-g>"] {
            press(&mut input, Scope::Logs, "g");
            assert!(matches!(
                press(&mut input, Scope::Logs, second),
                Match::Cancelled
            ));
            assert!(input.hints(Scope::Logs).is_none());
        }
        assert!(matches!(
            press(&mut input, Scope::Logs, "q"),
            Match::Execute(Action::App(AppAction::Quit))
        ));
    }

    #[test]
    fn a_second_key_can_also_be_a_global_single_key() {
        let maps = maps("logs: [{on: [g, q], action: Clear}]").unwrap();
        let mut input = Matcher::new(&maps);
        press(&mut input, Scope::Logs, "g");
        assert!(matches!(
            press(&mut input, Scope::Logs, "q"),
            Match::Execute(Action::Logs(logs::Action::Clear))
        ));
    }

    #[test]
    fn chords_started_from_help_keep_only_global_candidates() {
        let maps = maps("app: [{on: [g, p], action: TabPrev}]").unwrap();
        let mut input = Matcher::new(&maps);
        for second in ["g", "p"] {
            input.feed(&"g".parse().unwrap(), Scope::Logs, Scope::App);
            assert_eq!(
                hint_rows(&input.hints(Scope::Logs).unwrap()),
                vec![("p".into(), "Tab Previous".into())]
            );
            // Help has closed, but this sequence retains the map where it began.
            let result = press(&mut input, Scope::Logs, second);
            if second == "g" {
                assert!(matches!(result, Match::Cancelled));
            } else {
                assert!(matches!(
                    result,
                    Match::Execute(Action::App(AppAction::TabPrev))
                ));
            }
        }
    }

    #[test]
    fn all_displayed_bindings_execute_the_action_from_the_same_context_map() {
        let maps = maps("app: [{on: [g, p], action: TabPrev, desc: Previous page}]\nlogs: [{on: [g, e], action: Clear, desc: Clear by chord}]").unwrap();
        for scope in [
            Scope::App,
            Scope::Connections,
            Scope::Proxies,
            Scope::Srvctl,
            Scope::Settings,
            Scope::Logs,
            Scope::Profile,
            Scope::Template,
        ] {
            assert_eq!(
                maps.page_bindings(scope).len() + maps.global_bindings().len(),
                maps.effective_bindings(scope).len(),
            );
            for (_, binding) in maps.effective_bindings(scope) {
                let descriptions = if matches!(binding.action, Action::App(_)) {
                    docs(maps.global_bindings())
                } else {
                    docs(maps.page_bindings(scope))
                };
                assert!(descriptions.contains(&(binding.key_text.clone(), binding.desc.clone())));
                let mut input = Matcher::new(&maps);
                let mut result = Match::Unmatched;
                for key in binding.on.as_slice() {
                    result = input.feed(key, scope, scope);
                    if matches!(result, Match::Pending) {
                        assert!(
                            input
                                .hints(scope)
                                .unwrap()
                                .bindings()
                                .iter()
                                .any(|candidate| std::ptr::eq(*candidate, binding))
                        );
                    }
                }
                assert!(matches!(result, Match::Execute(action) if action == binding.action));
                assert!(input.hints(scope).is_none());
            }
            assert!(maps.effective_bindings(scope).any(
                |(_, binding)| binding.action == Action::App(AppAction::Tab(TabTarget::Status))
            ));
        }
    }
}
