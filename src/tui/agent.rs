use anyhow::{Result, bail};

pub fn init() -> Result<()> {
    let path = crate::config::keymap_path();

    if !path.exists() {
        log::debug!("Skip loading keymap");
        return Ok(());
    }

    let file = std::fs::File::open(path)?;
    let mut value: serde_yml::Mapping = serde_yml::from_reader(file)?;

    let mut keymap = get(&mut value, "keymap")?;

    super::tab::prelude::agent_init(&mut keymap)?;

    Ok(())
}

pub fn get(value: &mut serde_yml::Mapping, idx: &str) -> Result<serde_yml::Mapping> {
    let Some(maybe_map) = value.remove(idx) else {
        bail!("Does not contian `keymap` section")
    };
    let serde_yml::Value::Mapping(map) = maybe_map else {
        bail!("Section`keymap` is not mapping")
    };
    Ok(map)
}

#[test]
fn example() -> anyhow::Result<()> {
    use crossterm::event::*;
    use std::collections::HashMap;

    #[derive(serde::Deserialize, Debug)]
    enum Key {
        Select,
    }

    let str = r#"
keymap:
  file:
    profile:
      ? code: Enter
        modifiers: ''
        kind: Press
        state: ''
      : Select
"#;
    let value =
        serde_yml::from_str::<serde_yml::Mapping>(str)?["keymap"]["file"]["profile"].clone();
    let keymap: HashMap<KeyEvent, Key> = serde_yml::from_value(value)?;
    println!("{:?}", keymap);
    assert!(matches!(
        keymap.get(&KeyEvent::new(KeyCode::Enter, KeyModifiers::empty())),
        Some(Key::Select)
    ));
    Ok(())
}
