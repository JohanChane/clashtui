use crossterm::event::{KeyCode, KeyEvent as _KeyEvent, KeyModifiers};

pub type KeyDesc = Vec<(String, &'static str)>;
pub type KeyDescRef<'a> = &'a [(String, &'a str)];
pub type KeyMap<A> = std::collections::HashMap<Key, MaybeMap<A>>;

pub trait AsStaticStr {
    fn as_static_str(&self) -> &'static str;
}

pub enum MaybeMap<A> {
    SubMap(std::collections::HashMap<Key, A>),
    Action(A),
}

#[derive_aliases::derive(..KeyBasic, Debug)]
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

/// Build KeyMap
///
/// - load keymapping from file or something via `set`
/// - get keymapping via `get`
///
/// File Format:
///
/// `Act1: [Key1, Key2, ...]`
macro_rules! key_map {
    ($actid:ident, [$(($key:expr, $action:expr),)*]) => {
pub(in crate::tui) mod km {
    use super::*;
    use anyhow::Context;
    use std::sync::OnceLock;
    use crate::tui::key::{Key as _Key, KeyDesc, KeyMap as _KeyMap, AsStaticStr, MaybeMap, utils::*};

    type KeyMap = _KeyMap<$actid>;

    pub(super) static KEYMAP: OnceLock<KeyMap> = OnceLock::new();

    pub fn set(map: serde_yml::Value) -> anyhow::Result<bool> {
        let map = serde_yml::from_value(map).context("Failed to load keymap")?;
        let is_duplicated = check_duplicate(&map);
        if KEYMAP.set(map_from_file(map)).is_err() {
            unreachable!("keymap initiated twice");
        }
        Ok(is_duplicated)
    }

    pub fn get() -> &'static KeyMap {
        KEYMAP.get().expect("try get keymap without init")
    }

    pub fn default() -> FileMap<$actid> {
        // TODO: remove me when app is filled
        #[allow(unused_mut)]
        let mut map = FileMap {
            common: Default::default(),
            submap: Default::default(),
        };
        $(
            map.common.entry($action).or_default().push(_Key::from_code($key));
        )*
        map
    }

    pub fn get_docs() -> KeyDesc {
        use crate::tui::key::MaybeMap;
        std::iter::from_fn(|| {
            while let Some((key, maybe_submap)) = km::get().iter().next() {
                match maybe_submap {
                    MaybeMap::SubMap(hash_map) => {
                        while let Some((key, act)) = hash_map.iter().next() {
                            return Some((key.to_string(), act.as_static_str()));
                        }
                    }
                    MaybeMap::Action(act) => {
                        return Some((key.to_string(), act.as_static_str()));
                    }
                };
            }
            None
        })
        .collect()
    }

    impl TryFrom<&_Key> for $actid {
        type Error = ();

        fn try_from(ev: &_Key) -> Result<Self, Self::Error> {
            use std::sync::Mutex;

            static SUBMAP: Mutex<Option<&'static std::collections::HashMap<_Key, $actid>>> =
                Mutex::new(None);

            let maybe_submap = &mut *SUBMAP.lock().unwrap();
            let key = if let Some(submap) = maybe_submap.take() {
                submap.get(ev).copied().ok_or(())?
            } else {
                match km::get().get(ev).ok_or(())? {
                    MaybeMap::SubMap(hash_map) => {
                        *maybe_submap = Some(hash_map);
                        return Err(());
                    }
                    MaybeMap::Action(key) => *key,
                }
            };
            return Ok(key);
        }
    }
}};
}

pub fn load() -> anyhow::Result<()> {
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

    macro_rules! quick_load {
        ($rec:expr, files::$id:ident $(, $($rest:tt)*)?) => {
            if files::$id::km::set(value.remove(concat!("files/", stringify!($id))).unwrap())? {
                $rec.push(concat!("files/", stringify!($id)))
            }
            quick_load!($rec $(, $($rest)*)?);
        };
        ($rec:expr, $id:ident $(, $($rest:tt)*)?) => {
            if $id::km::set(value.remove(stringify!($id)).unwrap())? {
                $rec.push(stringify!($id))
            }
            quick_load!($rec $(, $($rest)*)?);
        };
        ($rec: expr) => {}
    }
    use super::tab::*;

    let mut has_duplicate = vec![];
    quick_load!(
        has_duplicate,
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
            $map.insert(concat!("files/", stringify!($id)).into(), serde_yml::to_value(files::$id::km::default())?);
            quick_default!($map $(, $($rest)*)?);
        };
        ($map:expr, $id:ident $(, $($rest:tt)*)?) => {
            $map.insert(stringify!($id).into(), serde_yml::to_value($id::km::default())?);
            quick_default!($map $(, $($rest)*)?);
        };
        ($map: expr) => {}
    }
    use super::tab::*;

    let mut map = serde_yml::Mapping::new();
    quick_default!(
        map,
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

pub mod utils {
    use super::{Key, KeyMap, MaybeMap};
    use std::collections::{HashMap, HashSet};
    use std::hash::Hash;

    #[derive(serde::Deserialize, serde::Serialize)]
    pub struct FileMap<A: Eq + Hash> {
        #[serde(flatten)]
        pub common: HashMap<A, Vec<Key>>,
        #[serde(flatten)]
        pub submap: HashMap<Key, HashMap<A, Vec<Key>>>,
    }

    #[test]
    fn test() {
        #[derive(PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        enum Action {
            Act1,
            Act2,
            Act3,
        }
        use crossterm::event::KeyCode;
        let mut map: FileMap<Action> = FileMap {
            common: Default::default(),
            submap: Default::default(),
        };
        map.common
            .insert(Action::Act1, vec![Key::from_code(KeyCode::BackTab)]);
        map.submap.insert(
            Key::from_code(KeyCode::Down),
            [
                (Action::Act3, vec![Key::from_code(KeyCode::Backspace)]),
                (Action::Act2, vec![Key::from_code(KeyCode::Delete)]),
            ]
            .into(),
        );
        map.submap.insert(
            Key::from_code(KeyCode::Up),
            [(Action::Act2, vec![Key::from_code(KeyCode::Char('A'))])].into(),
        );
        let str = serde_yml::to_string(&map).unwrap();
        println!("{str}")
    }

    /// If there is duplicate (e.g. 's' => Search/Import), return true
    pub fn check_duplicate<A: Eq + Hash>(map: &FileMap<A>) -> bool {
        let mut set = HashSet::new();
        let no_duplicate = map
            .common
            .values()
            .flat_map(|keys| keys.into_iter())
            .chain(map.submap.keys())
            .all(|key| set.insert(key));
        if !no_duplicate {
            return true;
        }
        for submap in map.submap.values() {
            let mut set = HashSet::new();
            let no_duplicate = submap
                .values()
                .flat_map(|keys| keys.into_iter())
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
            .map(|(key, submap)| {
                (
                    key,
                    MaybeMap::SubMap(
                        submap
                            .into_iter()
                            .flat_map(|(act, keys)| keys.into_iter().map(move |key| (key, act)))
                            .collect(),
                    ),
                )
            })
            .chain(iter)
            .collect()
    }
}

pub mod consts {
    pub const MOVE_UP: &'static str = "Move Up";
    pub const MOVE_DOWN: &'static str = "Move Down";
    pub const GO_TOP: &'static str = "Go to top";
    pub const GO_BOTTOM: &'static str = "Go to bottom";
    pub const FILTER: &'static str = "Search/Filter";
    pub const PAUSE: &'static str = "Pause/Resume";
}
