use crossterm::event::{KeyCode, KeyEvent as _KeyEvent, KeyModifiers};

pub type KeyDesc = Vec<(String, &'static str)>;
pub type KeyDescRef<'a> = &'a [(String, &'a str)];
pub type KeyMap<A> = std::collections::HashMap<Key, MaybeMap<A>>;

pub trait Document {
    fn get_doc(&self) -> &'static str;
}

pub enum MaybeMap<A> {
    SubMap {
        name: String,
        inner: std::collections::HashMap<Key, A>,
    },
    Action(A),
}

#[derive_aliases::derive(..Key, Debug)]
pub struct Key {
    /// The key itself.
    pub code: KeyCode,
    #[serde(
        default = "KeyModifiers::empty",
        skip_serializing_if = "KeyModifiers::is_empty"
    )]
    /// Additional key modifiers.
    pub modifiers: KeyModifiers,
}

impl Key {
    fn normalize(mut self) -> Self {
        let c = match self.code {
            KeyCode::Char(c) => c,
            KeyCode::BackTab => {
                return Self {
                    code: KeyCode::Tab,
                    modifiers: KeyModifiers::SHIFT,
                };
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

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut prefix = String::new();
        if self.modifiers.contains(KeyModifiers::ALT) {
            prefix.push('A');
        }
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            prefix.push('C');
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            prefix.push('S');
        }
        if prefix.is_empty() {
            write!(f, "{}", self.code)
        } else {
            write!(f, "{prefix}-{}", self.code)
        }
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

pub mod instancing {
    use super::{Document, Key, KeyDesc, KeyMap, MaybeMap};

    /// Generate `[(key, description 'about the key's action')]` at runtime
    pub fn make_docs<A: Document>(keymap: &KeyMap<A>) -> KeyDesc {
        use crate::tui::key::MaybeMap;
        let mut iter = keymap.iter();
        let mut inner_iter: Option<(Key, std::collections::hash_map::Iter<'_, Key, A>)> = None;
        std::iter::from_fn(|| {
            if let Some((key, inner_iter)) = inner_iter.as_mut()
                && let Some((key2, act)) = inner_iter.by_ref().next()
            {
                return Some((format!("{key}+{key2}"), act.get_doc()));
            }
            for (key, maybe_submap) in iter.by_ref() {
                match maybe_submap {
                    MaybeMap::SubMap { name: _, inner } => {
                        inner_iter = Some((*key, inner.iter()));
                        if let Some((key2, act)) = inner_iter.as_mut().unwrap().1.by_ref().next() {
                            return Some((format!("{key}+{key2}"), act.get_doc()));
                        }
                    }
                    MaybeMap::Action(act) => {
                        return Some((key.to_string(), act.get_doc()));
                    }
                };
            }
            None
        })
        .collect()
    }

    /// Try to match an `A` by lookup in `submap` or `keymap`
    pub fn try_from<'a, A: Eq + Copy>(
        ev: &Key,
        keymap: &'a KeyMap<A>,
        submap: &mut Option<(&'a str, &'a std::collections::HashMap<Key, A>)>,
    ) -> Option<A> {
        let key = if let Some(submap) = submap.take() {
            submap.1.get(ev).copied()?
        } else {
            match keymap.get(ev)? {
                MaybeMap::SubMap { name, inner } => {
                    *submap = Some((name, inner));
                    return None;
                }
                MaybeMap::Action(key) => *key,
            }
        };
        Some(key)
    }

    pub mod files {
        use super::{Key, KeyMap, MaybeMap};
        use std::collections::{HashMap, HashSet};
        use std::hash::Hash;

        #[derive(serde::Deserialize, serde::Serialize)]
        pub struct FileMap<A: Eq + Hash> {
            // #[serde(flatten)]
            common: HashMap<A, Vec<Key>>,
            // #[serde(flatten)]
            submap: HashMap<String, SubMap<A>>,
        }
        impl<A: Eq + Hash> FileMap<A> {
            pub fn new() -> Self {
                Self {
                    common: Default::default(),
                    submap: Default::default(),
                }
            }
            pub fn with_common(
                mut self,
                map: impl IntoIterator<Item = (impl Into<Key>, A)>,
            ) -> Self {
                map.into_iter()
                    .for_each(|(key, act)| self.common.entry(act).or_default().push(key.into()));
                self
            }
            pub fn with_submap(
                mut self,
                name: impl ToString,
                key: impl Into<Key>,
                map: impl IntoIterator<Item = (impl Into<Key>, A)>,
            ) -> Self {
                let inner = &mut self
                    .submap
                    .entry(name.to_string())
                    .or_insert(SubMap {
                        key: key.into(),
                        inner: Default::default(),
                    })
                    .inner;
                map.into_iter()
                    .for_each(|(key, act)| inner.entry(act).or_default().push(key.into()));
                self
            }
        }

        #[derive(serde::Deserialize, serde::Serialize)]
        pub struct SubMap<A: Eq + Hash> {
            key: Key,
            // #[serde(flatten)]
            inner: HashMap<A, Vec<Key>>,
        }

        /// If there is duplicate (e.g. 's' => Search/Import), return true
        pub fn check_duplicate<A: Eq + Hash>(map: &FileMap<A>) -> bool {
            let mut set = HashSet::new();
            let no_duplicate = map
                .common
                .values()
                .flat_map(|keys| keys.iter())
                .chain(map.submap.values().map(|submap| &submap.key))
                .all(|key| set.insert(key));
            if !no_duplicate {
                return true;
            }
            for submap in map.submap.values() {
                let mut set = HashSet::new();
                let no_duplicate = submap
                    .inner
                    .values()
                    .flat_map(|keys| keys.iter())
                    .all(|key| set.insert(key));
                if !no_duplicate {
                    return true;
                }
            }
            false
        }

        pub fn map_from_file<A: Eq + Hash + Copy>(map: FileMap<A>) -> KeyMap<A> {
            let iter = map.common.into_iter().flat_map(|(act, keys)| {
                keys.into_iter()
                    .map(move |key| (key, MaybeMap::Action(act)))
            });
            map.submap
                .into_iter()
                .map(|(name, submap)| {
                    (
                        submap.key,
                        MaybeMap::SubMap {
                            name,
                            inner: submap
                                .inner
                                .into_iter()
                                .flat_map(|(act, keys)| keys.into_iter().map(move |key| (key, act)))
                                .collect(),
                        },
                    )
                })
                .chain(iter)
                .collect()
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
/// `Act1: [Key1, Key2, ...]`
macro_rules! key_map {
    ($actid:ident, $default_map:expr) => {
        pub(in crate::tui) mod km {
            use super::*;
            use crate::tui::key::{
                Key as _Key, KeyDesc, KeyMap as _KeyMap,
                instancing::{files::*, *},
            };
            use anyhow::Context;
            use std::sync::OnceLock;

            type KeyMap = _KeyMap<$actid>;

            pub(super) static KEYMAP: OnceLock<KeyMap> = OnceLock::new();

            pub fn set(map: serde_yml::Value) -> anyhow::Result<bool> {
                let map = serde_yml::from_value(map).context("Failed to parse keymap")?;
                let is_duplicated = check_duplicate(&map);
                if KEYMAP.set(map_from_file(map)).is_err() {
                    unreachable!("keymap initiated twice");
                }
                Ok(is_duplicated)
            }

            pub(super) fn get() -> &'static KeyMap {
                KEYMAP.get().expect("try get keymap without init")
            }

            pub fn default() -> serde_yml::Result<serde_yml::Value> {
                serde_yml::to_value::<FileMap<$actid>>($default_map)
            }

            // For re-export like files.rs does
            pub(in super::super) fn get_docs() -> KeyDesc {
                make_docs(km::get())
            }

            static SUBMAP: std::sync::Mutex<
                Option<(
                    &'static str,
                    &'static std::collections::HashMap<_Key, $actid>,
                )>,
            > = std::sync::Mutex::new(None);

            // For re-export like app.rs does
            pub(in super::super) fn get_submap_name() -> Option<&'static str> {
                SUBMAP.lock().unwrap().map(|l| l.0)
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

    let mut value: serde_yml::Mapping = match std::fs::File::open(&path) {
        Ok(file) => serde_yml::from_reader(file)?,
        Err(e) => {
            return Err(anyhow::anyhow!(
                "failed to open keymap file at {}: {e}",
                path.display()
            ));
        }
    };

    macro_rules! quick_load {
        ($rec:expr, files::$id:ident $(, $($rest:tt)*)?) => {
            if files::$id::km::set(value.remove(concat!("files/", stringify!($id))).unwrap())
                .context(concat!("failed to load keymap for files/", stringify!($id)))? {
                $rec.push(concat!("files/", stringify!($id)))
            }
            quick_load!($rec $(, $($rest)*)?);
        };
        ($rec:expr, $id:ident $(, $($rest:tt)*)?) => {
            if $id::km::set(value.remove(stringify!($id)).unwrap())
                .context(concat!("failed to load keymap for ", stringify!($id)))? {
                $rec.push(stringify!($id))
            }
            quick_load!($rec $(, $($rest)*)?);
        };
        ($rec: expr) => {}
    }
    use super::{app, tab::*};
    use anyhow::Context;

    let mut has_duplicate = vec![];
    quick_load!(
        has_duplicate,
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

pub fn init() -> anyhow::Result<()> {
    macro_rules! quick_default {
        ($map:expr, files::$id:ident $(, $($rest:tt)*)?) => {
            $map.insert(concat!("files/", stringify!($id)).into(), files::$id::km::default()?);
            quick_default!($map $(, $($rest)*)?);
        };
        ($map:expr, $id:ident $(, $($rest:tt)*)?) => {
            $map.insert(stringify!($id).into(), $id::km::default()?);
            quick_default!($map $(, $($rest)*)?);
        };
        ($map: expr $(,)?) => {}
    }
    use super::{app, tab::*};

    let mut map = serde_yml::Mapping::new();
    quick_default!(
        map,
        app,
        connections,
        proxies,
        srvctl,
        settings,
        logs,
        files::profile,
        files::template
    );

    let path = crate::config::keymap_path();
    let file = std::fs::File::create(path)?;
    serde_yml::to_writer(file, &map)?;
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
