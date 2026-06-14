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
    pub fn from_code(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::empty(),
        }
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
    }
}

/// Build KeyMap
///
/// - load keymapping from file or something via `init`
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
    use std::collections::{HashMap, HashSet};
    use std::sync::OnceLock;
    use crate::tui::key::{Key as _Key, KeyDesc, KeyMap as _KeyMap, AsStaticStr};

    type KeyMap = _KeyMap<$actid>;
    type FileMap = HashMap<$actid, Vec<_Key>>;

    static KEYMAP: OnceLock<KeyMap> = OnceLock::new();

    pub fn init(map: serde_yml::Value) -> anyhow::Result<bool> {
        let map = serde_yml::from_value(map).context("Failed to load keymap")?;
        let is_duplicated = check_duplicate(&map);
        if KEYMAP.set(map_from_file(map)).is_err() {
            anyhow::bail!("")
        }
        Ok(is_duplicated)
    }

    pub fn get() -> &'static KeyMap {
        KEYMAP.get().expect("try get keymap without init")
    }

    pub fn default() -> FileMap {
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

    fn check_duplicate(map: &FileMap) -> bool {
        let it = map.values();
        let excepted = it.len();
        let got = it
            .scan(HashSet::new(), |set, val| set.insert(val).then_some(()))
            .count();
        excepted == got
    }

    fn map_from_file(map: FileMap) -> KeyMap {
        map.into_iter()
            .flat_map(|(act, keys)| keys.into_iter().map(move |key| (key, act)))
            .collect()
    }

    impl TryFrom<&_Key> for $actid {
        type Error = ();

        fn try_from(ev: &_Key) -> Result<Self, Self::Error> {
            let km = km::get();
            return km.get(ev).map(|act| *act).ok_or(());
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
        ($rec:expr, $id:ident) => {
            if $id::km::init(value.remove(stringify!($id)).unwrap())? {
                $rec.push(stringify!($id))
            }
        };
        ($rec:expr, $($id:ident),+ $(,)?) => {
            $(quick_load!($rec, $id);)+
        };
    }
    use super::tab::*;

    let mut has_duplicate = vec![];
    quick_load!(has_duplicate, connections, proxies, srvctl, settings, logs);
    files::km_init(&mut has_duplicate, value.remove("files").unwrap())?;

    Ok(())
}

pub fn init() -> anyhow::Result<()> {
    macro_rules! quick_default {
        ($map:expr, $id:ident) => {
            $map.insert(stringify!($id).into(), serde_yml::to_value($id::km::default())?);
        };
        ($map:expr, $($id:ident),+ $(,)?) => {
            $(quick_default!($map, $id);)+
        };
    }
    use super::tab::*;

    let mut map = serde_yml::Mapping::new();
    quick_default!(map, connections, proxies, srvctl, settings, logs);
    map.insert("files".into(), files::km_default()?);

    let path = crate::config::keymap_path();
    let file = std::fs::File::create(path)?;
    serde_yml::to_writer(file, &map)?;
    Ok(())
}

pub mod consts {
    pub const MOVE_UP: &'static str = "Move Up";
    pub const MOVE_DOWN: &'static str = "Move Down";
    pub const GO_TOP: &'static str = "Go to top";
    pub const GO_BOTTOM: &'static str = "Go to bottom";
    pub const FILTER: &'static str = "Search/Filter";
    pub const PAUSE: &'static str = "Pause/Resume";
}
