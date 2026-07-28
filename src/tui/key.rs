use crossterm::event::{KeyCode, KeyEvent as _KeyEvent, KeyModifiers};

pub type KeyDesc = Vec<(String, &'static str)>;
pub type KeyDescRef<'a> = &'a [(String, &'a str)];
pub type KeyMap<A> = std::collections::HashMap<Key, A>;

pub trait AsStaticStr {
    fn as_static_str(&self) -> &'static str;
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
    use crate::tui::key::{Key as _Key, KeyDesc, KeyMap as _KeyMap, AsStaticStr, utils::*};

    type KeyMap = _KeyMap<$actid>;

    static KEYMAP: OnceLock<KeyMap> = OnceLock::new();

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
        let mut map = FileMap::new();
        $(
            map.entry($action).or_default().push(_Key::from_code($key));
        )*
        map
    }

    pub fn get_docs() -> KeyDesc {
        get()
            .iter()
            .map(|(key, act)| (key.to_string(), act.as_static_str()))
            .collect()
    }

    impl TryFrom<&_Key> for $actid {
        type Error = ();

        fn try_from(ev: &_Key) -> Result<Self, Self::Error> {
            return km::get().get(ev).map(|act| *act).ok_or(());
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
    use super::{Key, KeyMap};
    use std::collections::{HashMap, HashSet};

    pub type FileMap<K> = HashMap<K, Vec<Key>>;

    /// If there is duplicate (e.g. 's' => Search/Import), return true
    pub fn check_duplicate<K>(map: &FileMap<K>) -> bool {
        let it = map.values();
        let expected = it.len();
        // Any duplicate will cause got less than expected
        let got = it
            .scan(HashSet::new(), |set, val| set.insert(val).then_some(()))
            .count();
        expected != got
    }

    pub fn map_from_file<K: Copy>(map: FileMap<K>) -> KeyMap<K> {
        map.into_iter()
            .flat_map(|(act, keys)| keys.into_iter().map(move |key| (key, act)))
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
