use super::*;
use crossterm::event::KeyEvent;
use instancing::files::FileMap;
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

fn defaults() -> FileMap<Action> {
    FileMap::new()
        .with_common([
            (KeyCode::Char('j'), Action::MoveDown),
            (KeyCode::Char('k'), Action::MoveUp),
        ])
        .with_submap(
            "Nav",
            KeyCode::Char('g'),
            [(KeyCode::Char('g'), Action::GoTop)],
        )
}

fn compile(yaml: &str) -> anyhow::Result<(KeyMap<Action>, KeyDesc)> {
    config::merge(defaults(), serde_yml::from_str(yaml)?)
}

fn resolve(map: &KeyMap<Action>, keys: &[&str]) -> Option<Action> {
    let mut pending = None;
    let mut result = None;
    for key in keys {
        result = instancing::try_from(&key.parse().unwrap(), map, &mut pending);
    }
    assert!(
        pending.is_none(),
        "test sequence must finish or cancel the chord"
    );
    result
}

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
fn chords_and_modified_keys_dispatch_without_copying_candidates() {
    let (map, docs) = compile("- on: [g, e]\n  action: Clear\n  desc: Clear by chord\n- on: [\"<C-g>\", \"<A-x>\"]\n  action: MoveDown\n- on: \"<C-j>\"\n  action: MoveDown").unwrap();
    assert_eq!(resolve(&map, &["g", "e"]), Some(Action::Clear));
    assert_eq!(resolve(&map, &["g", "g"]), Some(Action::GoTop));
    assert_eq!(resolve(&map, &["<C-g>", "<A-x>"]), Some(Action::MoveDown));
    assert_eq!(resolve(&map, &["<C-j>"]), Some(Action::MoveDown));
    assert_eq!(resolve(&map, &["j"]), Some(Action::MoveDown));
    assert!(docs.contains(&("g e".into(), "Clear by chord".into())));
    assert!(matches!(&map[&"g".parse().unwrap()], MaybeMap::SubMap {name, ..} if name == "Nav"));
}

#[test]
fn replacing_a_chord_changes_action_and_description() {
    let (map, docs) = compile("- on: [g, g]\n  action: Clear\n  desc: Custom chord").unwrap();
    assert_eq!(resolve(&map, &["g", "g"]), Some(Action::Clear));
    assert_eq!(docs.iter().filter(|(key, _)| key == "g g").count(), 1);
    assert!(docs.contains(&("g g".into(), "Custom chord".into())));
}

#[test]
fn unmatched_second_key_clears_pending_state() {
    let (map, _) = compile("[]").unwrap();
    assert_eq!(resolve(&map, &["g", "x"]), None);
    assert_eq!(resolve(&map, &["j"]), Some(Action::MoveDown));
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
fn same_keys_in_different_scopes_are_allowed() {
    assert!(compile("- {on: x, action: Clear}").is_ok());
    assert!(compile("- {on: x, action: MoveUp}").is_ok());
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
fn nested_dev_action_format_is_supported() {
    #[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize)]
    enum Nested {
        Action(Action),
    }
    impl Document for Nested {
        fn get_doc(&self) -> &'static str {
            "Nested action"
        }
    }
    let overrides: Value = serde_yml::from_str("- on: x\n  action:\n    Action: Clear").unwrap();
    let (map, _) = config::merge(FileMap::<Nested>::new(), overrides).unwrap();
    assert!(matches!(
        map[&"x".parse().unwrap()],
        MaybeMap::Action(Nested::Action(Action::Clear))
    ));
}

#[test]
fn earlier_submap_configuration_remains_readable() {
    let value = serde_yml::to_value(defaults()).unwrap();
    let (map, _) = config::merge(FileMap::<Action>::new(), value).unwrap();
    assert_eq!(resolve(&map, &["j"]), Some(Action::MoveDown));
    assert_eq!(resolve(&map, &["g", "g"]), Some(Action::GoTop));
}

#[test]
fn core_specific_file_scopes_override_only_the_selected_core() {
    let yaml = "logs: [{on: x, action: Clear}]\nfile:\n  profile: [{on: e, action: {Action: Edit}}]\nmihomo:\n  logs: [{on: y, action: Clear}]\n  file:\n    template: [{on: p, action: {Action: Preview}}]\nsing-box:\n  logs: [{on: z, action: Clear}]";
    let input = serde_yml::from_str(yaml).unwrap();
    let map = config::sections(input, "mihomo").unwrap();
    assert!(map.contains_key("files/profile"));
    assert!(map.contains_key("files/template"));
    let (compiled, _) = config::merge(defaults(), map["logs"].clone()).unwrap();
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
    ] {
        assert!(config::sections(serde_yml::from_str(yaml).unwrap(), "mihomo").is_err());
    }
}

#[test]
fn notation_matches_crossterm_events_across_shift_conventions() {
    for c in ['G', '?', '!', '~', ':', ' '] {
        let unix = Key::from(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        let windows = Key::from(KeyEvent::new(KeyCode::Char(c), KeyModifiers::SHIFT));
        assert_eq!(unix, windows);
        assert_eq!(c.to_string().parse::<Key>().unwrap(), unix);
    }
    assert_eq!("<S-g>".parse::<Key>().unwrap(), "G".parse().unwrap());
    assert_eq!("<S-?>".parse::<Key>().unwrap(), "?".parse().unwrap());
    assert_eq!(
        Key::from(KeyEvent::new(
            KeyCode::BackTab,
            KeyModifiers::CONTROL | KeyModifiers::SHIFT
        )),
        "<C-S-Tab>".parse().unwrap()
    );
}

#[test]
fn key_notation_round_trips() {
    for text in [
        "j",
        "G",
        "?",
        "中",
        " ",
        "<",
        ">",
        "<C-u>",
        "<A-x>",
        "<D-x>",
        "<C-S-a>",
        "<S-Up>",
        "<BackTab>",
        "<C-->",
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
    let (map, docs) = compile("- {on: j, action: null}\n- {on: [j, j], action: Clear}").unwrap();
    assert_eq!(resolve(&map, &["j", "j"]), Some(Action::Clear));
    assert!(!docs.iter().any(|(key, _)| key == "j"));
    let (map, _) = compile("- {on: [g, g], action: null}\n- {on: g, action: Clear}").unwrap();
    assert_eq!(resolve(&map, &["g"]), Some(Action::Clear));
    let (map, _) = compile("- {on: j, action: null}").unwrap();
    assert_eq!(resolve(&map, &["j"]), None);
    assert_eq!(resolve(&map, &["k"]), Some(Action::MoveUp));
}

#[test]
fn dev_structured_key_format_retains_modifiers_and_descriptions() {
    let yaml = "? code: !Char x\n  ctrl: true\n: action: Clear\n  desc: Custom control binding\n";
    let (map, docs) = compile(yaml).unwrap();
    assert_eq!(resolve(&map, &["<C-x>"]), Some(Action::Clear));
    assert_eq!(resolve(&map, &["x"]), None);
    assert!(docs.contains(&("<C-x>".into(), "Custom control binding".into())));
}

#[test]
fn all_real_scopes_accept_the_bundled_user_configuration_and_legacy_defaults() {
    crate::tui::app::km::km::validate_config(crate::tui::app::km::default().unwrap()).unwrap();
    use crate::tui::tab::{connections, files, logs, proxies, settings, srvctl};
    let mut bundled = config::sections(
        serde_yml::from_str(include_str!(
            "../../../contrib/default_configs/default_keymap.yaml"
        ))
        .unwrap(),
        "mihomo",
    )
    .unwrap();
    macro_rules! check {
        ($module:path, $scope:literal) => {{
            use $module as km;
            km::validate_config(Value::Null).unwrap();
            km::validate_config(km::default().unwrap()).unwrap();
            km::validate_config(bundled.remove($scope).unwrap())
                .unwrap_or_else(|e| panic!("{}: {e:#}", $scope));
        }};
    }
    check!(connections::km, "connections");
    check!(proxies::km, "proxies");
    check!(logs::km, "logs");
    check!(settings::km, "settings");
    check!(srvctl::km, "srvctl");
    check!(files::profile::km, "files/profile");
    check!(files::template::km, "files/template");
    assert!(bundled.is_empty());
}
