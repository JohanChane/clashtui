use anyhow::Result;
use std::io;
use std::str::FromStr;

// ---- OnVisitor / deserialize_on (handles "j" / "<C-u>" / ["g","g"], rejects empty) ----

struct OnVisitor;

impl<'de> serde::de::Visitor<'de> for OnVisitor {
    type Value = Vec<crate::tui::Key>;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a key string or list of key strings (e.g. \"j\" or [\"g\", \"g\"])")
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
        if v.is_empty() {
            return Err(serde::de::Error::custom(
                "empty key string; on must not be empty",
            ));
        }
        let key = crate::tui::Key::from_str(v).map_err(serde::de::Error::custom)?;
        Ok(vec![key])
    }

    fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
        self.visit_str(&v)
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut keys = Vec::new();
        while let Some(s) = seq.next_element::<String>()? {
            let key = crate::tui::Key::from_str(&s).map_err(serde::de::Error::custom)?;
            keys.push(key);
        }
        if keys.is_empty() {
            return Err(serde::de::Error::custom(
                "empty key sequence; on must not be empty",
            ));
        }
        Ok(keys)
    }
}

pub fn deserialize_on<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Vec<crate::tui::Key>, D::Error> {
    d.deserialize_any(OnVisitor)
}

// ---- RawEntry (replaces old Entry) ----

/// User YAML entry -- action is deferred (each scope deserializes its own type).
#[derive(serde::Deserialize)]
pub struct RawEntry {
    #[serde(deserialize_with = "deserialize_on")]
    pub on: Vec<crate::tui::Key>,
    pub exec: serde_yml::Value,
    #[serde(default)]
    pub desc: Option<String>,
}

// ---- init ----

pub fn init() -> Result<()> {
    let path = crate::config::keymap_path();

    let mut value: serde_yml::Mapping = match std::fs::File::open(&path) {
        Ok(file) => serde_yml::from_reader(file)?,
        Err(e) if e.kind() == io::ErrorKind::NotFound => serde_yml::Mapping::new(),
        Err(e) => {
            return Err(anyhow::anyhow!(
                "failed to open keymap file at {}: {e}",
                path.display()
            ));
        }
    };

    // No more core-specific section merging (mihomo:/sing-box: deleted)
    super::tab::prelude::agent_init(&mut value)?;

    Ok(())
}

// ---- Tests ----

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_entry_rejects_empty_on_string() {
        let yaml = r#"
on: ""
exec: MoveDown
"#;
        let result: Result<Vec<RawEntry>, _> = serde_yml::from_str(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_raw_entry_rejects_empty_on_array() {
        let yaml = r#"
on: []
exec: MoveDown
"#;
        let result: Result<Vec<RawEntry>, _> = serde_yml::from_str(yaml);
        assert!(result.is_err());
    }
}
