use crossterm::event::{KeyCode, KeyEvent as _KeyEvent, KeyModifiers};

pub(super) mod config;
mod notation;

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
    use super::{Key, KeyCode, KeyMap, MaybeMap, SubMap};

    /// Try to match an `A` by lookup in `submap` or `keymap`
    pub fn try_from<'a, A: Eq + Copy>(
        ev: &Key,
        keymap: &'a KeyMap<A>,
        submap: &mut Option<&'a SubMap<A>>,
    ) -> Option<A> {
        let key = if let Some(submap) = submap.take() {
            if *ev == Key::from_code(KeyCode::Esc) {
                return None;
            }
            submap.inner.get(ev).copied()?
        } else {
            match keymap.get(ev)? {
                MaybeMap::SubMap(inner) => {
                    *submap = Some(inner);
                    return None;
                }
                MaybeMap::Action(key) => *key,
            }
        };
        Some(key)
    }

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

/// Build KeyMap
///
/// - load keymapping from file or something via `set`
/// - get keymapping via `get`
///
/// File Format:
///
/// User entries: `[{on: "j", action: MoveDown, desc: "Move down"}, ...]`.
macro_rules! key_map {
    ($actid:ident, $default_map:expr) => {
        pub(in crate::tui) mod km {
            use super::*;
            use crate::tui::key::{Key as _Key, KeyDesc, KeyMap as _KeyMap, instancing::*};
            use std::sync::OnceLock;

            type KeyMap = _KeyMap<$actid>;

            pub(super) static KEYMAP: OnceLock<(KeyMap, KeyDesc)> = OnceLock::new();

            pub fn set(map: serde_yml::Value) -> anyhow::Result<()> {
                let configured = crate::tui::key::config::merge($default_map, map)?;
                if KEYMAP.set(configured).is_err() {
                    anyhow::bail!("keymap initiated twice");
                }
                Ok(())
            }

            #[cfg(test)]
            pub fn validate_config(map: serde_yml::Value) -> anyhow::Result<KeyDesc> {
                crate::tui::key::config::merge::<$actid>($default_map, map).map(|(_, docs)| docs)
            }

            pub(super) fn get() -> &'static KeyMap {
                &KEYMAP.get().expect("try get keymap without init").0
            }

            // For re-export like files.rs does
            pub(in super::super) fn get_docs() -> KeyDesc {
                // Generate [(key, description about the key's action)] once at startup.
                KEYMAP.get().expect("try get keymap without init").1.clone()
            }

            pub(super) static SUBMAP: std::sync::Mutex<
                Option<&'static crate::tui::key::SubMap<$actid>>,
            > = std::sync::Mutex::new(None);

            // For re-export like app.rs does
            pub(in super::super) fn get_submap_name() -> Option<&'static str> {
                pending_keys().map(|hints| hints.name.as_str())
            }

            pub(in super::super) fn pending_keys() -> Option<&'static crate::tui::key::WhichHints> {
                SUBMAP.lock().unwrap().map(|submap| &submap.hints)
            }

            pub(in super::super) fn cancel_pending() {
                *SUBMAP.lock().unwrap() = None;
            }

            pub(in super::super) fn has_binding(key: &_Key) -> bool {
                get().contains_key(key)
            }

            pub(in super::super) fn starts_chord(key: &_Key) -> bool {
                matches!(get().get(key), Some(crate::tui::key::MaybeMap::SubMap(_)))
            }

            impl TryFrom<&_Key> for $actid {
                type Error = ();

                fn try_from(ev: &_Key) -> Result<Self, Self::Error> {
                    let maybe_submap = &mut *SUBMAP.lock().unwrap();
                    try_from(ev, km::get(), maybe_submap).ok_or(())
                }
            }
        }
    };
}

pub fn load() -> anyhow::Result<()> {
    let path = crate::config::keymap_path();
    let core = match crate::config::CONFIG.core_type() {
        crate::config::CoreType::Mihomo => "mihomo",
        crate::config::CoreType::Singbox => "sing-box",
    };
    let mut value = config::sections(read_keymap(&path)?, core)?;

    macro_rules! quick_load {
        (files::$id:ident $(, $($rest:tt)*)?) => {
            files::$id::km::set(value.remove(concat!("files/", stringify!($id))).unwrap_or_default())
                .context(concat!("failed to load keymap for files/", stringify!($id)))?;
            quick_load!($($($rest)*)?);
        };
        ($id:ident $(, $($rest:tt)*)?) => {
            $id::km::set(value.remove(stringify!($id)).unwrap_or_default())
                .context(concat!("failed to load keymap for ", stringify!($id)))?;
            quick_load!($($($rest)*)?);
        };
        () => {}
    }
    use super::{app, tab::*};
    use anyhow::Context;

    quick_load!(
        app,
        connections,
        proxies,
        srvctl,
        settings,
        logs,
        files::profile,
        files::template
    );

    Ok(())
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
