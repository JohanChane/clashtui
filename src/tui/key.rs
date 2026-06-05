use crossterm::event::KeyEvent;
pub type Key = KeyEvent;

/// Build KeyMap
///
/// - load keymapping from file or something via `init`
/// - get keymapping via `get`
///
/// File Format:
///
/// `Act1: [Key1, Key2, ...]`
macro_rules! key_map {
    ($actid:ident, [$(($key:expr, $action:expr, $desc:literal),)*]) => {
pub mod km {
    use super::*;
    use anyhow::Context;
    use std::collections::{HashMap, HashSet};
    use std::sync::OnceLock;
    use crate::tui::key::Key as _Key;

    pub type KeyMap = HashMap<_Key, $actid>;
    type FileMap = HashMap<$actid, Vec<_Key>>;

    static MAPPING: OnceLock<KeyMap> = OnceLock::new();

    pub fn init(map: serde_yml::Value) -> anyhow::Result<bool> {
        let map = serde_yml::from_value(map).context("Failed to load keymap")?;
        let is_duplicated = check_duplicate(&map);
        if MAPPING.set(map_from_file(map)).is_err() {
            anyhow::bail!("")
        }
        Ok(is_duplicated)
    }

    pub fn get() -> &'static KeyMap {
        MAPPING.get().expect("try get keymap without init")
    }

    pub fn default() -> FileMap {
        let mut map = FileMap::new();
        $(
            map.entry($action).or_default().push($key.into());
        )*
        map
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
    use super::{app, tab::*};

    let mut has_duplicate = vec![];
    quick_load!(
        has_duplicate,
        app,
        connections,
        proxies,
        srvctl,
        settings,
        logs
    );
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
    use super::{app, tab::*};

    let mut map = serde_yml::Mapping::new();
    quick_default!(map, app, connections, proxies, srvctl, settings, logs);
    map.insert("files".into(), files::km_default()?);

    let path = crate::config::keymap_path();
    let file = std::fs::File::create(path)?;
    serde_yml::to_writer(file, &map)?;
    Ok(())
}
