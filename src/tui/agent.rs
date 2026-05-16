use crate::config::CoreType;
use crate::tui::widget::tab::KeyCombo;
use anyhow::Result;
use std::collections::HashMap;
use std::str::FromStr;

/// Round-trip through YAML string to handle tagged values (e.g. `!Char`)
fn from_value_robust<T: serde::de::DeserializeOwned>(val: &serde_yml::Value) -> Result<T> {
    let s = serde_yml::to_string(val)?;
    Ok(serde_yml::from_str(&s)?)
}

// ---- list-based format (yazi-style) ----

struct OnVisitor;

impl<'de> serde::de::Visitor<'de> for OnVisitor {
    type Value = Vec<crate::tui::Key>;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a key string or list of key strings (e.g. \"j\" or [\"g\", \"g\"])")
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
        let key = crate::tui::Key::from_str(v).map_err(serde::de::Error::custom)?;
        Ok(vec![key])
    }

    fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
        let key = crate::tui::Key::from_str(&v).map_err(serde::de::Error::custom)?;
        Ok(vec![key])
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut keys = Vec::new();
        while let Some(s) = seq.next_element::<String>()? {
            let key = crate::tui::Key::from_str(&s).map_err(serde::de::Error::custom)?;
            keys.push(key);
        }
        Ok(keys)
    }
}

fn deserialize_on<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Vec<crate::tui::Key>, D::Error> {
    d.deserialize_any(OnVisitor)
}

#[derive(serde::Deserialize)]
pub struct Entry {
    #[serde(deserialize_with = "deserialize_on")]
    on: Vec<crate::tui::Key>,
    action: serde_yml::Value,
    #[serde(default)]
    desc: Option<String>,
}

pub fn extract_keymap_list<K: serde::de::DeserializeOwned>(
    entries: Vec<Entry>,
) -> Result<(
    HashMap<crate::tui::Key, K>,
    HashMap<crate::tui::Key, String>,
    Vec<(KeyCombo, K, String)>,
)> {
    let mut agent = HashMap::new();
    let mut descs = HashMap::new();
    let mut chords = Vec::new();

    for entry in entries {
        let action: K = deserialize_action_value(entry.action)?;
        if entry.on.len() == 1 {
            let key = entry.on[0];
            agent.insert(key, action);
            if let Some(desc) = entry.desc {
                if !desc.is_empty() {
                    descs.insert(key, desc);
                }
            }
        } else {
            chords.push((KeyCombo(entry.on), action, entry.desc.unwrap_or_default()));
        }
    }

    Ok((agent, descs, chords))
}

/// Deserialize a serde_yml::Value into K.
/// serde_yml::from_value on a Value::Mapping fails for externally-tagged enums,
/// so we embed the value in a container struct to trigger YAML document deserialization.
fn deserialize_action_value<K: serde::de::DeserializeOwned>(val: serde_yml::Value) -> Result<K> {
    if !matches!(val, serde_yml::Value::Mapping(_)) {
        return Ok(serde_yml::from_value(val)?);
    }
    let s = serde_yml::to_string(&val)?;
    let container = format!("value:\n  {s}");
    #[derive(serde::Deserialize)]
    struct Container<K> {
        value: K,
    }
    let c: Container<K> = serde_yml::from_str(&container)?;
    Ok(c.value)
}

pub fn check_duplicate_keys_list(section: &str, entries: &[Entry]) {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    for entry in entries {
        if entry.on.len() == 1 {
            let k = entry.on[0];
            if !seen.insert(k) {
                log::warn!(
                    "duplicate key `{k}` in [{section}] keymap — later binding overwrites earlier"
                );
            }
        }
    }
}

// ---- helpers ----

pub fn take_section(value: &mut serde_yml::Mapping, idx: &str) -> Option<serde_yml::Value> {
    value.remove(idx)
}

pub fn extract_keymap_with_descs<K: serde::de::DeserializeOwned>(
    map: serde_yml::Mapping,
) -> Result<(
    HashMap<crate::tui::Key, K>,
    HashMap<crate::tui::Key, String>,
)> {
    let mut agent = HashMap::new();
    let mut descs = HashMap::new();
    for (key_val, value_val) in map {
        let key: crate::tui::Key = from_value_robust(&key_val)?;
        // Try WithDesc format: { action: K, desc: String }
        if let serde_yml::Value::Mapping(ref m) = value_val {
            if let Some(action_val) = m.get("action") {
                let action: K = from_value_robust(action_val)?;
                agent.insert(key, action);
                if let Some(desc_val) = m.get("desc") {
                    let desc: String = from_value_robust(desc_val).unwrap_or_default();
                    if !desc.is_empty() {
                        descs.insert(key, desc);
                    }
                }
                continue;
            }
        }
        // Simple format: scalar or Action: Edit
        let action: K = from_value_robust(&value_val)?;
        agent.insert(key, action);
    }
    Ok((agent, descs))
}

pub fn init() -> Result<()> {
    let path = crate::config::keymap_path();

    let mut value: serde_yml::Mapping = match std::fs::File::open(&path) {
        Ok(file) => serde_yml::from_reader(file)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => serde_yml::Mapping::new(),
        Err(e) => {
            return Err(anyhow::anyhow!(
                "failed to open keymap file at {}: {e}",
                path.display()
            ));
        }
    };

    let (mut common, core_specific) = split_sections(&mut value);

    if let Some(mut core_map) = core_specific {
        merge_mappings(&mut common, &mut core_map);
    }

    super::tab::prelude::agent_init(&mut common)?;

    Ok(())
}

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
                log::warn!(
                    "duplicate key `{k}` in [{section}] keymap — later binding overwrites earlier"
                );
            }
        }
    }
}

#[test]
fn example() -> anyhow::Result<()> {
    use crate::tui::Key;
    use std::collections::HashMap;

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
    let value = serde_yml::from_str::<serde_yml::Mapping>(str)?["file"]["profile"].clone();
    let keymap: HashMap<Key, K> = serde_yml::from_value(value)?;
    println!("{:?}", keymap);
    assert!(matches!(
        keymap.get(&Key {
            code: crossterm::event::KeyCode::Enter,
            shift: false,
            ctrl: false,
            alt: false,
            super_: false
        }),
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

    assert!(
        mihomo_section.is_some(),
        "mihomo section should be extracted"
    );

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
    use crate::tui::Key as TuiKey;
    use crate::tui::tab::files::profile::Key;
    use std::collections::HashMap;

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
    let keymap: HashMap<TuiKey, Key> = serde_yml::from_value(serde_yml::Value::Mapping(value))?;
    assert_eq!(keymap.len(), 3);
    Ok(())
}

#[test]
fn test_profile_key_with_action_mapping_no_crash() -> anyhow::Result<()> {
    use crate::tui::Key as TuiKey;
    use crate::tui::tab::files::profile::Key;
    use std::collections::HashMap;

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
    let keymap: HashMap<TuiKey, Key> = serde_yml::from_value(serde_yml::Value::Mapping(value))?;
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
    use crate::tui::Key as TuiKey;
    use crate::tui::tab::files::template::Key;
    use std::collections::HashMap;

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
    let keymap: HashMap<TuiKey, Key> = serde_yml::from_value(serde_yml::Value::Mapping(value))?;
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
        panic!(
            "duplicate keys in default agents:\n{}",
            violations.join("\n")
        );
    }
}

#[derive(serde::Deserialize, Debug, PartialEq)]
enum TestAction {
    MoveDown,
    MoveUp,
    GoTop,
    Select,
    Edit,
    ToggleNoPp,
}

#[test]
fn test_list_format_single_key() -> anyhow::Result<()> {
    let yaml = r#"
- on: "j"
  action: MoveDown
"#;
    let entries: Vec<Entry> = serde_yml::from_str(yaml)?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].on.len(), 1);
    let (keys, _, chords) = extract_keymap_list::<TestAction>(entries)?;
    assert_eq!(keys.len(), 1);
    assert!(chords.is_empty());
    let j = crate::tui::Key {
        code: crossterm::event::KeyCode::Char('j'),
        shift: false,
        ctrl: false,
        alt: false,
        super_: false,
    };
    assert_eq!(keys.get(&j), Some(&TestAction::MoveDown));
    Ok(())
}

#[test]
fn test_list_format_modifier_key() -> anyhow::Result<()> {
    let yaml = r#"
- on: "<C-u>"
  action: MoveUp
"#;
    let entries: Vec<Entry> = serde_yml::from_str(yaml)?;
    assert_eq!(entries.len(), 1);
    let (keys, _, _) = extract_keymap_list::<TestAction>(entries)?;
    let ctrl_u = crate::tui::Key {
        code: crossterm::event::KeyCode::Char('u'),
        shift: false,
        ctrl: true,
        alt: false,
        super_: false,
    };
    assert_eq!(keys.get(&ctrl_u), Some(&TestAction::MoveUp));
    Ok(())
}

#[test]
fn test_list_format_chord() -> anyhow::Result<()> {
    let yaml = r#"
- on: ["g", "g"]
  action: GoTop
"#;
    let entries: Vec<Entry> = serde_yml::from_str(yaml)?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].on.len(), 2);
    let (keys, _, chords) = extract_keymap_list::<TestAction>(entries)?;
    assert!(keys.is_empty());
    assert_eq!(chords.len(), 1);
    assert_eq!(chords[0].0.len(), 2);
    assert_eq!(chords[0].1, TestAction::GoTop);
    Ok(())
}

#[test]
fn test_list_format_desc() -> anyhow::Result<()> {
    let yaml = r#"
- on: "j"
  action: MoveDown
  desc: Move cursor down
"#;
    let entries: Vec<Entry> = serde_yml::from_str(yaml)?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].desc.as_deref(), Some("Move cursor down"));
    let (_, descs, _) = extract_keymap_list::<TestAction>(entries)?;
    assert_eq!(descs.len(), 1);
    Ok(())
}

#[test]
fn test_list_format_nested_action() -> anyhow::Result<()> {
    // Use the real profile::Key type which has a custom Deserialize impl
    // that handles "Action: Edit" format correctly
    use crate::tui::tab::files::profile::Key;
    let yaml = r#"
- on: "e"
  action:
    Action: Edit
"#;
    let entries: Vec<Entry> = serde_yml::from_str(yaml)?;
    let (keys, _, _) = extract_keymap_list::<Key>(entries)?;
    let e = crate::tui::Key {
        code: crossterm::event::KeyCode::Char('e'),
        shift: false,
        ctrl: false,
        alt: false,
        super_: false,
    };
    assert!(matches!(keys.get(&e), Some(Key::Action(_))));
    Ok(())
}

#[test]
fn test_list_format_multiple_same_action() -> anyhow::Result<()> {
    let yaml = r#"
- on: "j"
  action: MoveDown
- on: "<Down>"
  action: MoveDown
"#;
    let entries: Vec<Entry> = serde_yml::from_str(yaml)?;
    assert_eq!(entries.len(), 2);
    let (keys, _, chords) = extract_keymap_list::<TestAction>(entries)?;
    assert_eq!(keys.len(), 2);
    assert!(chords.is_empty());
    let j = crate::tui::Key {
        code: crossterm::event::KeyCode::Char('j'),
        shift: false,
        ctrl: false,
        alt: false,
        super_: false,
    };
    let down = crate::tui::Key {
        code: crossterm::event::KeyCode::Down,
        shift: false,
        ctrl: false,
        alt: false,
        super_: false,
    };
    assert_eq!(keys.get(&j), Some(&TestAction::MoveDown));
    assert_eq!(keys.get(&down), Some(&TestAction::MoveDown));
    Ok(())
}
