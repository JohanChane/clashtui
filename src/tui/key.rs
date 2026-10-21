use crossterm::event::{KeyCode, KeyEvent as _KeyEvent, KeyModifiers};

pub(super) mod config;
mod effective;
mod matcher;
pub(super) mod notation;

pub(in crate::tui) use effective::{Action, EffectiveKeymap, Keymaps, Scope};
pub use matcher::{Match, Matcher};

#[cfg(test)]
mod tests;

pub type KeyDesc = Vec<(String, String)>;
pub type KeyDescRef<'a> = &'a [(String, String)];
pub type KeyMap<A> = std::collections::HashMap<Key, MaybeMap<A>>;

pub trait Document {
    fn get_doc(&self) -> &'static str;
}

pub enum MaybeMap<A> {
    SubMap(SubMap<A>),
    Action(A),
}

pub struct SubMap<A> {
    pub inner: std::collections::HashMap<Key, A>,
    pub hints: WhichHints,
}

/// Presentation data compiled once from the same bindings as the dispatch map.
#[derive(Clone)]
pub struct WhichHints {
    pub prefix: Key,
    pub name: String,
    pub keys: KeyDesc,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Key {
    /// The key itself.
    pub code: KeyCode,
    /// Additional key modifiers.
    pub modifiers: KeyModifiers,
}

impl Key {
    fn normalize(mut self) -> Self {
        let c = match self.code {
            KeyCode::Char(c) => c,
            KeyCode::BackTab => {
                self.code = KeyCode::Tab;
                self.modifiers.insert(KeyModifiers::SHIFT);
                return self;
            }
            _ => return self,
        };
        if c.is_ascii_uppercase() {
            // 大写字母 → 自动加上 SHIFT modifier
            self.modifiers.insert(KeyModifiers::SHIFT);
        } else if self.modifiers.contains(KeyModifiers::SHIFT) {
            // SHIFT + 小写字母 → 转为大写字母
            self.code = KeyCode::Char(c.to_ascii_uppercase())
        }
        // Character events already contain shifted punctuation on both platforms.
        // Windows may additionally report SHIFT; Unix terminals usually do not.
        if matches!(self.code, KeyCode::Char(c) if !c.is_ascii_uppercase()) {
            self.modifiers.remove(KeyModifiers::SHIFT);
        }
        self
    }
    pub fn from_code(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::empty(),
        }
        .normalize()
    }
    // pub fn with_modifiers(code: KeyCode, modifiers: KeyModifiers) -> Self {
    //     Self { code, modifiers }
    // }
}

impl From<_KeyEvent> for Key {
    fn from(value: _KeyEvent) -> Self {
        Self {
            code: value.code,
            modifiers: value.modifiers,
        }
        .normalize()
    }
}

impl From<KeyCode> for Key {
    fn from(value: KeyCode) -> Self {
        Self::from_code(value)
    }
}

pub mod instancing {
    use super::Key;

    pub mod defaults {
        use super::Key;
        use crate::tui::key::config::Binding;

        /// Built-in bindings in declaration order; no configuration serialization needed.
        pub struct DefaultMap<A> {
            bindings: Vec<Binding<A>>,
        }

        impl<A> DefaultMap<A> {
            pub fn new() -> Self {
                Self {
                    bindings: Vec::new(),
                }
            }

            pub fn with_common(
                mut self,
                map: impl IntoIterator<Item = (impl Into<Key>, A)>,
            ) -> Self {
                self.bindings
                    .extend(map.into_iter().map(|(key, action)| Binding {
                        on: vec![key.into().normalize()],
                        action: Some(action),
                        desc: None,
                        group: None,
                    }));
                self
            }

            pub fn with_submap(
                mut self,
                name: impl ToString,
                key: impl Into<Key>,
                map: impl IntoIterator<Item = (impl Into<Key>, A)>,
            ) -> Self {
                let name = name.to_string();
                let first = key.into().normalize();
                self.bindings
                    .extend(map.into_iter().map(|(key, action)| Binding {
                        on: vec![first, key.into().normalize()],
                        action: Some(action),
                        desc: None,
                        group: Some(name.clone()),
                    }));
                self
            }

            pub fn into_bindings(self) -> Vec<Binding<A>> {
                self.bindings
            }
        }
    }
}

/// Declare a scope's typed defaults and configuration parser.
/// User entries: `[{on: "j", action: MoveDown, desc: "Move down"}, ...]`.
macro_rules! key_map {
    ($scope:ident, $actid:ident, $default_map:expr) => {
        pub(in crate::tui) mod km {
            use super::*;

            pub(in crate::tui) fn bindings(
                map: serde_yml::Value,
            ) -> anyhow::Result<
                Vec<crate::tui::key::config::ResolvedBinding<crate::tui::key::Action>>,
            > {
                crate::tui::key::config::resolve::<$actid>($default_map, map).map(|bindings| {
                    bindings
                        .into_iter()
                        .map(|binding| binding.map_action(crate::tui::key::Action::$scope))
                        .collect()
                })
            }

            #[cfg(test)]
            pub fn validate_config(
                map: serde_yml::Value,
            ) -> anyhow::Result<crate::tui::key::KeyDesc> {
                bindings(map).map(|bindings| crate::tui::key::config::compile(bindings).1)
            }
        }
    };
}

static KEYMAPS: std::sync::OnceLock<Keymaps> = std::sync::OnceLock::new();

pub(in crate::tui) fn keymaps() -> &'static Keymaps {
    KEYMAPS
        .get()
        .expect("keymaps must be loaded before starting the TUI")
}

pub fn load() -> anyhow::Result<()> {
    use anyhow::Context;
    let path = crate::config::keymap_path();
    let core = match crate::config::CONFIG.core_type() {
        crate::config::CoreType::Mihomo => "mihomo",
        crate::config::CoreType::Singbox => "sing-box",
    };
    let maps = Keymaps::from_config(read_keymap(&path)?, core)
        .with_context(|| format!("failed to load keymap at {}", path.display()))?;
    KEYMAPS
        .set(maps)
        .map_err(|_| anyhow::anyhow!("keymaps initialized twice"))
}

fn read_keymap(path: &std::path::Path) -> anyhow::Result<serde_yml::Mapping> {
    match std::fs::File::open(path) {
        Ok(file) => serde_yml::from_reader::<_, Option<serde_yml::Mapping>>(file)
            .map(Option::unwrap_or_default)
            .map_err(|e| anyhow::anyhow!("failed to parse keymap file at {}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(serde_yml::Mapping::new()),
        Err(e) => Err(anyhow::anyhow!(
            "failed to open keymap file at {}: {e}",
            path.display()
        )),
    }
}

pub fn init() -> anyhow::Result<()> {
    std::fs::write(
        crate::config::keymap_path(),
        "# Optional overrides; unlisted bindings keep their defaults.\n\
         # Example: logs: [{on: x, action: Clear, desc: Clear logs}]\n{}\n",
    )?;
    Ok(())
}

pub mod consts {
    pub const MOVE_UP: &str = "Move Up";
    pub const MOVE_DOWN: &str = "Move Down";
    pub const GO_TOP: &str = "Go to top";
    pub const GO_BOTTOM: &str = "Go to bottom";
    pub const FILTER: &str = "Search/Filter";
    pub const PAUSE: &str = "Pause/Resume";
}
