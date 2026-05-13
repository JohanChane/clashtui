use anyhow::Result;
use crate::config::CoreType;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Write;

fn fmt_key(k: &crate::tui::Key) -> String {
    use crossterm::event::KeyCode;
    let mut s = String::new();
    s.push_str("{ code:");
    match k.code {
        KeyCode::Char(c) => write!(s, " !Char {c}").unwrap(),
        other => write!(s, " {other:?}").unwrap(),
    }
    if k.shift {
        s.push_str(", shift: true");
    }
    if k.ctrl {
        s.push_str(", ctrl: true");
    }
    if k.alt {
        s.push_str(", alt: true");
    }
    if k.super_ {
        s.push_str(", super_: true");
    }
    s.push_str(" }");
    s
}

fn append_section<K: serde::Serialize>(
    out: &mut String,
    name: &str,
    indent: &str,
    shortcuts: &[(crate::tui::widget::tab::KeyCombo, K, &str)],
) {
    writeln!(out, "{indent}{name}:").unwrap();
    let sub = format!("{indent}  ");
    for (combo, action, desc) in shortcuts {
        if combo.len() != 1 {
            continue;
        }
        let key = combo[0];

        // Serialize action to YAML fragment
        let action_s = serde_yml::to_string(action).unwrap();
        let action_yaml = action_s.trim();
        let is_scalar = !action_yaml.contains(':') && !action_yaml.contains('\n');

        if desc.is_empty() && is_scalar {
            // Simple format — value inline after :
            //   ? code: !Char k
            //   : MoveUp
            out.push_str(&format!("{sub}? "));
            out.push_str(&fmt_key(&key));
            writeln!(out, "\n{sub}: {action_yaml}").unwrap();
        } else {
            // Block format — value is a mapping { action, desc }
            //   ? code: !Char k
            //   :
            //     action: MoveUp
            //     desc: Move up
            out.push_str(&format!("{sub}? "));
            out.push_str(&fmt_key(&key));
            writeln!(out, "\n{sub}:").unwrap();
            if is_scalar {
                writeln!(out, "{sub}  action: {action_yaml}").unwrap();
            } else {
                writeln!(out, "{sub}  action:").unwrap();
                for line in action_yaml.lines() {
                    writeln!(out, "{sub}    {line}").unwrap();
                }
            }
            if !desc.is_empty() {
                writeln!(out, "{sub}  desc: {desc}").unwrap();
            }
        }
    }
}

fn generate_default_keymap_yaml() -> String {
    use crate::tui::tab;

    let mut out = String::new();
    out.push_str("# Clashtui keymap — auto-generated\n");
    out.push_str("# Add entries here to override default key bindings.\n");
    out.push_str("# Press ? in the TUI to see current bindings with descriptions.\n");
    out.push_str("# Core-specific sections (mihomo / sing-box) override common sections.\n\n");

    append_section(&mut out, "connections", "", tab::connections::all_shortcuts());
    append_section(&mut out, "proxies", "", tab::proxies::all_shortcuts());

    out.push_str("file:\n");
    append_section(&mut out, "profile", "  ", tab::files::profile::all_shortcuts());
    append_section(&mut out, "template", "  ", tab::files::template::all_shortcuts());

    append_section(&mut out, "srvctl", "", tab::srvctl::all_shortcuts());
    append_section(&mut out, "settings", "", tab::settings::all_shortcuts());
    append_section(&mut out, "logs", "", tab::logs::all_shortcuts());

    out
}

/// Round-trip through YAML string to handle tagged values (e.g. `!Char`)
fn from_value_robust<T: serde::de::DeserializeOwned>(val: &serde_yml::Value) -> Result<T> {
    let s = serde_yml::to_string(val)?;
    Ok(serde_yml::from_str(&s)?)
}

pub fn extract_keymap_with_descs<K: serde::de::DeserializeOwned>(
    map: serde_yml::Mapping,
) -> Result<(HashMap<crate::tui::Key, K>, HashMap<crate::tui::Key, String>)> {
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

fn generate_default_keymap(path: &std::path::Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, generate_default_keymap_yaml())?;
    Ok(())
}

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
fn test_generated_keymap_has_all_sections() {
    let yaml = generate_default_keymap_yaml();
    let value: serde_yml::Mapping = serde_yml::from_str(&yaml).unwrap();
    assert!(value.contains_key("connections"));
    assert!(value.contains_key("proxies"));
    assert!(value.contains_key("file"));
    assert!(value.contains_key("srvctl"));
    assert!(value.contains_key("settings"));
    assert!(value.contains_key("logs"));

    let file = value
        .get("file")
        .and_then(|v| v.as_mapping())
        .expect("file should be a mapping");
    assert!(file.contains_key("profile"));
    assert!(file.contains_key("template"));
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
fn test_generated_keymap_entries_have_desc() {
    let yaml = generate_default_keymap_yaml();
    let value: serde_yml::Mapping = serde_yml::from_str(&yaml).unwrap();

    // Verify connections entries have desc
    let conns = value["connections"].as_mapping().unwrap();
    for (_, v) in conns {
        let m = v.as_mapping().expect("each entry should be a mapping");
        assert!(m.contains_key("action"), "entry missing action field");
        assert!(m.contains_key("desc"), "entry missing desc field");
    }
}

#[test]
fn test_generated_keymap_key_format_no_false_defaults() {
    use crate::tui::Key as TuiKey;

    let yaml = generate_default_keymap_yaml();
    let value: serde_yml::Mapping = serde_yml::from_str(&yaml).unwrap();

    // Parse back all keys — they should deserialize without false fields
    for (k, _) in &value {
        if k.as_str() == Some("file") {
            let file = value[k].as_mapping().unwrap();
            for (_, v) in file {
                let m = v.as_mapping().unwrap();
                for (key_val, _) in m {
                    let _key: TuiKey = serde_yml::from_value(key_val.clone()).unwrap();
                }
            }
        } else {
            let m = value[k].as_mapping().unwrap();
            for (key_val, _) in m {
                let _key: TuiKey = serde_yml::from_value(key_val.clone()).unwrap();
            }
        }
    }
}

#[test]
fn test_generated_keymap_deserializes_simple_sections() -> anyhow::Result<()> {
    use std::collections::HashMap;
    use crate::tui::Key as TuiKey;

    let yaml = generate_default_keymap_yaml();
    let mut value: serde_yml::Mapping = serde_yml::from_str(&yaml)?;

    // Verify simple-key sections parse correctly via extract_keymap_with_descs
    {
        let conns = get(&mut value.clone(), "connections")?;
        let (keys, descs) = extract_keymap_with_descs::<crate::tui::tab::connections::Key>(conns)?;
        assert!(!keys.is_empty());
        assert_eq!(keys.len(), descs.len());
    }
    {
        let srv = get(&mut value.clone(), "srvctl")?;
        let (keys, descs) = extract_keymap_with_descs::<crate::tui::tab::srvctl::SrvCtlKey>(srv)?;
        assert!(!keys.is_empty());
        assert_eq!(keys.len(), descs.len());
    }
    {
        let sett = get(&mut value.clone(), "settings")?;
        let (keys, descs) = extract_keymap_with_descs::<crate::tui::tab::settings::SettingsKey>(sett)?;
        assert!(!keys.is_empty());
        assert_eq!(keys.len(), descs.len());
    }
    {
        let lgs = get(&mut value.clone(), "logs")?;
        let (keys, descs) = extract_keymap_with_descs::<crate::tui::tab::logs::Key>(lgs)?;
        assert!(!keys.is_empty());
        assert_eq!(keys.len(), descs.len());
    }

    // file sections have nested structure — check they exist
    let file = get(&mut value.clone(), "file")?;
    assert!(file.contains_key("profile"));
    assert!(file.contains_key("template"));
    assert!(!file.get("profile").unwrap().as_mapping().unwrap().is_empty());
    assert!(!file.get("template").unwrap().as_mapping().unwrap().is_empty());

    Ok(())
}
