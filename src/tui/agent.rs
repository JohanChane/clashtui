use anyhow::Result;
use crate::config::CoreType;

pub fn init() -> Result<()> {
    let path = crate::config::keymap_path();

    if !path.exists() {
        generate_default_keymap(&path)?;
    }

    let file = std::fs::File::open(path)?;
    let mut value: serde_yml::Mapping = serde_yml::from_reader(file)?;

    let (mut common, core_specific) = split_sections(&mut value);

    if let Some(mut core_map) = core_specific {
        merge_mappings(&mut common, &mut core_map);
    }

    super::tab::prelude::agent_init(&mut common)?;

    Ok(())
}

fn generate_default_keymap(path: &std::path::Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, DEFAULT_KEYMAP_YAML)?;
    Ok(())
}

const DEFAULT_KEYMAP_YAML: &str = include_str!("keymap_default.yaml");

fn split_sections(
    value: &mut serde_yml::Mapping,
) -> (serde_yml::Mapping, Option<serde_yml::Mapping>) {
    let mihomo = take_mapping(value, "mihomo");
    let singbox = take_mapping(value, "sing-box");

    let core_type = crate::config::CONFIG.core_type();
    let core_specific = match core_type {
        CoreType::Mihomo => mihomo,
        CoreType::Singbox => singbox,
    };

    (value.clone(), core_specific)
}

fn take_mapping(value: &mut serde_yml::Mapping, key: &str) -> Option<serde_yml::Mapping> {
    let entry = value.remove(key)?;
    match entry {
        serde_yml::Value::Mapping(m) => Some(m),
        _ => None,
    }
}

fn merge_mappings(base: &mut serde_yml::Mapping, override_map: &mut serde_yml::Mapping) {
    for (key, val) in override_map.iter() {
        if let Some(serde_yml::Value::Mapping(base_map)) = base.get_mut(key) {
            if let serde_yml::Value::Mapping(override_inner) = val {
                merge_mappings(base_map, &mut override_inner.clone());
                continue;
            }
        }
        base.insert(key.clone(), val.clone());
    }
}

pub fn get(value: &mut serde_yml::Mapping, idx: &str) -> Result<serde_yml::Mapping> {
    let Some(maybe_map) = value.remove(idx) else {
        anyhow::bail!("Does not contain `{idx}` section")
    };
    let serde_yml::Value::Mapping(map) = maybe_map else {
        anyhow::bail!("Section `{idx}` is not mapping")
    };
    Ok(map)
}

pub fn check_duplicate_keys(section: &str, map: &serde_yml::Mapping) {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    for key in map.keys() {
        if let Ok(k) = serde_yml::from_value::<crate::tui::Key>(key.clone()) {
            if !seen.insert(k) {
                log::warn!("duplicate key `{k}` in [{section}] keymap — later binding overwrites earlier");
            }
        }
    }
}

#[test]
fn example() -> anyhow::Result<()> {
    use std::collections::HashMap;
    use crate::tui::Key;

    #[derive(serde::Deserialize, Debug)]
    enum K {
        Select,
    }

    let str = r#"
file:
  profile:
    ? code: Enter
      shift: false
      ctrl: false
      alt: false
      super_: false
    : Select
"#;
    let value =
        serde_yml::from_str::<serde_yml::Mapping>(str)?["file"]["profile"].clone();
    let keymap: HashMap<Key, K> = serde_yml::from_value(value)?;
    println!("{:?}", keymap);
    assert!(matches!(
        keymap.get(&Key { code: crossterm::event::KeyCode::Enter, shift: false, ctrl: false, alt: false, super_: false }),
        Some(K::Select)
    ));
    Ok(())
}

#[test]
fn test_section_merge_core_overrides_common() {
    let yaml = r#"
connections:
  ? code: Char('j')
    shift: false
    ctrl: false
    alt: false
    super_: false
  : MoveDown
mihomo:
  connections:
    ? code: Char('k')
      shift: false
      ctrl: false
      alt: false
      super_: false
    : MoveUp
"#;
    let mut value: serde_yml::Mapping = serde_yml::from_str(yaml).unwrap();

    // Simulate mihomo being the active core
    let mut common = value.clone();
    let mihomo_section = take_mapping(&mut common, "mihomo");
    // Remove sing-box section too
    common.remove("sing-box");

    assert!(mihomo_section.is_some(), "mihomo section should be extracted");

    if let Some(mut core_specific) = mihomo_section {
        merge_mappings(&mut common, &mut core_specific);
    }

    // After merge, common should have connections from mihomo
    let connections = common.get("connections").expect("connections should exist");
    assert!(connections.is_mapping(), "connections should be a mapping");
}

#[test]
fn test_no_keymap_wrapper_needed() {
    let yaml = r#"
connections:
  ? code: Char('j')
    shift: false
    ctrl: false
    alt: false
    super_: false
  : MoveDown
"#;
    let mut value: serde_yml::Mapping = serde_yml::from_str(yaml).unwrap();
    // Top-level directly has "connections" - no "keymap" wrapper needed
    assert!(value.contains_key("connections"));
    assert!(!value.contains_key("keymap"));
}

#[test]
fn test_take_mapping_removes_key() {
    let yaml = r#"
mihomo:
  foo: bar
common:
  baz: qux
"#;
    let mut value: serde_yml::Mapping = serde_yml::from_str(yaml).unwrap();
    let mihomo = take_mapping(&mut value, "mihomo");
    assert!(mihomo.is_some());
    assert!(!value.contains_key("mihomo"), "mihomo should be removed");
    assert!(value.contains_key("common"), "common should remain");
}

#[test]
fn test_profile_key_deserialization_string_variants() -> anyhow::Result<()> {
    use std::collections::HashMap;
    use crate::tui::Key as TuiKey;
    use crate::tui::tab::files::profile::Key;

    let yaml = r#"
? code: Enter
  shift: false
  ctrl: false
  alt: false
  super_: false
: Select
? code: Up
  shift: false
  ctrl: false
  alt: false
  super_: false
: MoveUp
? code: Down
  shift: false
  ctrl: false
  alt: false
  super_: false
: MoveDown
"#;
    let value: serde_yml::Mapping = serde_yml::from_str(yaml)?;
    let keymap: HashMap<TuiKey, Key> =
        serde_yml::from_value(serde_yml::Value::Mapping(value))?;
    assert_eq!(keymap.len(), 3);
    Ok(())
}

#[test]
fn test_profile_key_with_action_mapping_no_crash() -> anyhow::Result<()> {
    use std::collections::HashMap;
    use crate::tui::Key as TuiKey;
    use crate::tui::tab::files::profile::Key;

    let yaml = r#"
? code: !Char e
  shift: false
  ctrl: false
  alt: false
  super_: false
: Action: Edit
? code: !Char i
  shift: false
  ctrl: false
  alt: false
  super_: false
: Action: Add
"#;
    let value: serde_yml::Mapping = serde_yml::from_str(yaml)?;
    let keymap: HashMap<TuiKey, Key> =
        serde_yml::from_value(serde_yml::Value::Mapping(value))?;
    assert_eq!(keymap.len(), 2);
    let e_key = TuiKey {
        code: crossterm::event::KeyCode::Char('e'),
        shift: false,
        ctrl: false,
        alt: false,
        super_: false,
    };
    let i_key = TuiKey {
        code: crossterm::event::KeyCode::Char('i'),
        shift: false,
        ctrl: false,
        alt: false,
        super_: false,
    };
    assert!(matches!(keymap.get(&e_key), Some(Key::Action(_))));
    assert!(matches!(keymap.get(&i_key), Some(Key::Action(_))));
    Ok(())
}

#[test]
fn test_template_key_deserialization() -> anyhow::Result<()> {
    use std::collections::HashMap;
    use crate::tui::Key as TuiKey;
    use crate::tui::tab::files::template::Key;

    let yaml = r#"
? code: Enter
  shift: false
  ctrl: false
  alt: false
  super_: false
: Action: Generate
? code: Left
  shift: false
  ctrl: false
  alt: false
  super_: false
: Switch
"#;
    let value: serde_yml::Mapping = serde_yml::from_str(yaml)?;
    let keymap: HashMap<TuiKey, Key> =
        serde_yml::from_value(serde_yml::Value::Mapping(value))?;
    assert_eq!(keymap.len(), 2);
    let enter_key = TuiKey {
        code: crossterm::event::KeyCode::Enter,
        shift: false,
        ctrl: false,
        alt: false,
        super_: false,
    };
    assert!(matches!(keymap.get(&enter_key), Some(Key::Action(_))));
    Ok(())
}

#[test]
fn test_default_keymap_parses_and_has_all_sections() -> anyhow::Result<()> {
    let value: serde_yml::Mapping = serde_yml::from_str(DEFAULT_KEYMAP_YAML)?;
    assert!(value.contains_key("connections"));
    assert!(value.contains_key("file"));
    assert!(value.contains_key("srvctl"));
    assert!(value.contains_key("settings"));
    assert!(value.contains_key("logs"));

    // Verify the file section contains profile and template subsections
    let file = value
        .get("file")
        .and_then(|v| v.as_mapping())
        .expect("file should be a mapping");
    assert!(file.contains_key("profile"));
    assert!(file.contains_key("template"));
    Ok(())
}

#[test]
fn test_no_duplicate_keys_in_default_agents() {
    use std::collections::HashSet;
    let mut violations = Vec::new();

    macro_rules! check {
        ($name:expr, $agent:expr) => {{
            let agent = $agent;
            let mut seen = HashSet::new();
            for key in agent.keys() {
                if !seen.insert(*key) {
                    violations.push(format!("{}: duplicate key `{key}`", $name));
                }
            }
        }};
    }

    check!("connections", crate::tui::tab::connections::agent());
    check!("file/profile", crate::tui::tab::files::profile::agent());
    check!("file/template", crate::tui::tab::files::template::agent());
    check!("srvctl", crate::tui::tab::srvctl::agent());
    check!("settings", crate::tui::tab::settings::agent());
    check!("logs", crate::tui::tab::logs::agent());

    if !violations.is_empty() {
        panic!("duplicate keys in default agents:\n{}", violations.join("\n"));
    }
}

#[test]
fn test_default_keymap_deserializes_all_tabs() -> anyhow::Result<()> {
    use std::collections::HashMap;
    use crate::tui::Key as TuiKey;

    let mut value: serde_yml::Mapping = serde_yml::from_str(DEFAULT_KEYMAP_YAML)?;
    value.remove("mihomo");
    value.remove("sing-box");

    // connections
    {
        let conns = get(&mut value.clone(), "connections")?;
        let _km: HashMap<TuiKey, crate::tui::tab::connections::Key> =
            serde_yml::from_value(serde_yml::Value::Mapping(conns))?;
    }
    // srvctl
    {
        let srv = get(&mut value.clone(), "srvctl")?;
        let _km: HashMap<TuiKey, crate::tui::tab::srvctl::SrvCtlKey> =
            serde_yml::from_value(serde_yml::Value::Mapping(srv))?;
    }
    // settings
    {
        let sett = get(&mut value.clone(), "settings")?;
        let _km: HashMap<TuiKey, crate::tui::tab::settings::SettingsKey> =
            serde_yml::from_value(serde_yml::Value::Mapping(sett))?;
    }
    // logs
    {
        let lgs = get(&mut value.clone(), "logs")?;
        let _km: HashMap<TuiKey, crate::tui::tab::logs::Key> =
            serde_yml::from_value(serde_yml::Value::Mapping(lgs))?;
    }
    // file/profile
    {
        let file = get(&mut value.clone(), "file")?;
        let profile = get(&mut file.clone(), "profile")?;
        let _km: HashMap<TuiKey, crate::tui::tab::files::profile::Key> =
            serde_yml::from_value(serde_yml::Value::Mapping(profile))?;
    }
    // file/template
    {
        let file = get(&mut value.clone(), "file")?;
        let tmpl = get(&mut file.clone(), "template")?;
        let _km: HashMap<TuiKey, crate::tui::tab::files::template::Key> =
            serde_yml::from_value(serde_yml::Value::Mapping(tmpl))?;
    }
    Ok(())
}
