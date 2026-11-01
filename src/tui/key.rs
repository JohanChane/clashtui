use crossterm::event::{KeyCode, KeyEvent as _KeyEvent, KeyModifiers};

pub(super) mod config;
mod defaults;
mod matcher;
pub(super) mod notation;
mod runtime;

pub(in crate::tui) use defaults::DefaultBindings;
pub(in crate::tui) use matcher::{Match, Matcher};
pub(in crate::tui) use runtime::{Action, BindingView, ChordView, Keymaps, Scope};

#[cfg(test)]
mod tests;

pub trait Document {
    fn get_doc(&self) -> &'static str;
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
        // Preserve Shift on other characters; do not infer a keyboard layout.
        self
    }
    pub fn from_code(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::empty(),
        }
        .normalize()
    }
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

/// A shortcut is one key or a two-key chord in every stage of keymap loading.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(in crate::tui) enum KeySequence {
    Single(Key),
    Chord([Key; 2]),
}

impl KeySequence {
    pub fn as_slice(&self) -> &[Key] {
        match self {
            Self::Single(key) => std::slice::from_ref(key),
            Self::Chord(keys) => keys,
        }
    }

    fn validate_action(self) -> anyhow::Result<()> {
        anyhow::ensure!(
            !matches!(self, Self::Chord([_, second]) if second == Key::from_code(KeyCode::Esc)),
            "Esc is reserved for cancelling a pending chord; choose another second key"
        );
        Ok(())
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
    for scope in Scope::ALL {
        log::debug!("keymap {}:\n{}", scope.name(), maps.describe(scope));
    }
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
