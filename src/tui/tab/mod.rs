mod dev {
    pub use crate::tui::Key as TuiKey;
    pub use crate::tui::widget::dualtab::*;
    pub use crate::tui::widget::tab::*;
    pub use crossterm::event::KeyCode;
    pub use ratatui::prelude::{Frame, Rect};
    pub use ratatui::style::{Color, Stylize as _};
    pub use ratatui::widgets::{Block, List, ListState, StatefulWidget};

    pub use crate::tui::popmsg::prelude::*;
    pub(crate) use crate::tui::theme::Theme;
}

use crate::tui::widget::tab::KeyCombo;

macro_rules! tri {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => {
                crate::tui::widget::popmsg::Confirm::err(e);
                return do_nothing();
            }
        }
    };
    ($e:expr, or_cancel) => {
        match $e {
            Ok(v) => v,
            Err(_) => {
                return do_nothing();
            }
        }
    };
    ($e:expr, or_set) => {
        match $e {
            Ok(v) => v,
            Err(e) => {
                return wrapper(move |content: &mut Self| {
                    content.error = Some(e.to_string());
                });
            }
        }
    };
}

macro_rules! mod_agent {
    ($ident:ident, [$($tokens:tt)*]) => {
pub(crate) mod agent {
    use super::*;
    use std::collections::HashMap;
    use std::sync::OnceLock;

    pub type Agent = HashMap<crate::tui::Key, $ident>;

    static AGENT: OnceLock<Agent> = OnceLock::new();

    fn key_from_str(s: &str) -> crate::tui::Key {
        use std::str::FromStr;
        crate::tui::Key::from_str(s).expect("invalid key string in mod_agent!")
    }

    fn quick_map(code: KeyCode) -> crate::tui::Key {
        crate::tui::Key {
            code,
            shift: matches!(code, KeyCode::Char(c) if c.is_ascii_uppercase()),
            ctrl: false,
            alt: false,
            super_: false,
        }
    }

    fn default_agent() -> Agent {
        let mut m = Agent::new();
        mod_agent!(@agent m, $($tokens)*);
        m
    }

    pub fn agent() -> &'static Agent {
        AGENT.get_or_init(default_agent)
    }

    pub fn init(map: Agent) {
        if AGENT.set(map).is_err() {
            unreachable!("KeyMap Init Twice!")
        }
    }

    static DESC_OVERRIDES: OnceLock<HashMap<crate::tui::Key, String>> = OnceLock::new();

    pub fn init_descs(map: HashMap<crate::tui::Key, String>) {
        if !map.is_empty() {
            let _ = DESC_OVERRIDES.set(map);
        }
    }

    static USER_CHORDS: OnceLock<Vec<(KeyCombo, $ident, String)>> = OnceLock::new();

    pub fn init_chords(chords: Vec<(KeyCombo, $ident, String)>) {
        if !chords.is_empty() {
            let _ = USER_CHORDS.set(chords);
        }
    }

    static SHORTCUTS: OnceLock<Vec<(KeyCombo, $ident, &'static str)>> = OnceLock::new();

    pub fn all_shortcuts() -> &'static [(KeyCombo, $ident, &'static str)] {
        SHORTCUTS.get_or_init(|| {
            // Build default shortcuts from macro tokens
            let mut default_v: Vec<(KeyCombo, $ident, &'static str)> = Vec::new();
            mod_agent!(@shortcuts default_v, $($tokens)*);

            // Build default desc map for single-key entries
            let default_descs: HashMap<crate::tui::Key, &'static str> = default_v
                .iter()
                .filter(|(c, _, _)| c.len() == 1)
                .map(|(c, _, d)| (c[0], *d))
                .collect();

            let mut v = Vec::new();

            // If user config was loaded (init() already called), replace
            // single-key shortcuts with entries from the user's agent,
            // keeping chord shortcuts from defaults.
            if let Some(agent) = AGENT.get() {
                // Keep chord shortcuts from defaults
                for (combo, action, desc) in &default_v {
                    if combo.len() != 1 {
                        v.push((combo.clone(), *action, *desc));
                    }
                }
                // Add single-key entries from the user's agent
                for (key, action) in agent.iter() {
                    let desc = default_descs.get(key).copied().unwrap_or("");
                    v.push((KeyCombo(vec![*key]), *action, desc));
                }
            } else {
                v = default_v;
            }

            // Apply desc overrides from YAML
            if let Some(overrides) = DESC_OVERRIDES.get() {
                for (combo, _, desc) in &mut v {
                    if combo.len() == 1 {
                        if let Some(d) = overrides.get(&combo[0]) {
                            *desc = Box::leak(d.clone().into_boxed_str());
                        }
                    }
                }
            }

            // Add user-defined chords
            if let Some(user_chords) = USER_CHORDS.get() {
                if !user_chords.is_empty() {
                    // Remove default chords whose key sequence conflicts with user chords
                    let user_combos: std::collections::HashSet<KeyCombo> =
                        user_chords.iter().map(|(c, _, _)| c.clone()).collect();
                    v.retain(|(combo, _, _)| combo.len() == 1 || !user_combos.contains(combo));
                }
                for (combo, key, desc) in user_chords {
                    v.push((combo.clone(), *key, Box::leak(desc.clone().into_boxed_str())));
                }
            }

            v
        })
    }
}

pub use agent::{agent, all_shortcuts};
pub use agent::{init as agent_init, init_descs, init_chords};
    };

    // ---- tt-muncher: agent (only single-key shortcuts) ----
    (@agent $m:ident, ([$code:expr], $map:expr, $desc:expr) $($rest:tt)*) => {
        $m.insert(quick_map($code), $map);
        mod_agent!(@agent $m, $($rest)*);
    };
    (@agent $m:ident, (key($s:literal), $map:expr, $desc:expr) $($rest:tt)*) => {
        $m.insert(key_from_str($s), $map);
        mod_agent!(@agent $m, $($rest)*);
    };
    (@agent $m:ident, ([$($codes:expr),+], $map:expr, $desc:expr) $($rest:tt)*) => {
        mod_agent!(@agent $m, $($rest)*);
    };
    (@agent $m:ident, , $($rest:tt)*) => {
        mod_agent!(@agent $m, $($rest)*);
    };
    (@agent $m:ident,) => {};

    // ---- tt-muncher: shortcuts (both single-key and chords) ----
    (@shortcuts $v:ident, ([$($codes:expr),+], $map:expr, $desc:expr) $($rest:tt)*) => {
        $v.push((KeyCombo(vec![$(quick_map($codes)),+]), $map, $desc));
        mod_agent!(@shortcuts $v, $($rest)*);
    };
    (@shortcuts $v:ident, (key($s:literal), $map:expr, $desc:expr) $($rest:tt)*) => {
        $v.push((KeyCombo(vec![key_from_str($s)]), $map, $desc));
        mod_agent!(@shortcuts $v, $($rest)*);
    };
    (@shortcuts $v:ident, , $($rest:tt)*) => {
        mod_agent!(@shortcuts $v, $($rest)*);
    };
    (@shortcuts $v:ident,) => {};
}

macro_rules! newtype_tab {
    ($(#[$m:meta])* $tab:ident($ty:ident<$inner:ident>)) => {
        $(#[$m])*
        #[derive(Default)]
        pub struct $tab($ty<$inner>);

        crate::new_type_impl_tuiwidget!($tab);

        impl crate::tui::tab::TuiTab for $tab {
            fn title(&self) -> &'static str {
                $inner::TITLE
            }

            fn shortcuts(&self) -> &[(KeyCombo, &'static str)] {
                self.0.shortcuts()
            }

            fn dispatch_shortcut(&mut self, seq: &[crate::tui::Key]) {
                self.0.dispatch_shortcut(seq)
            }
        }
    };
    ($(#[$m:meta])* $tab:ident($inner:ty), $title:literal) => {
        $(#[$m])*
        #[derive(Default)]
        pub struct $tab($inner);

        crate::new_type_impl_tuiwidget!($tab);

        impl crate::tui::tab::TuiTab for $tab {
            fn title(&self) -> &'static str {
                $title
            }

            fn shortcuts(&self) -> &[(KeyCombo, &'static str)] {
                self.0.shortcuts()
            }

            fn dispatch_shortcut(&mut self, seq: &[crate::tui::Key]) {
                self.0.dispatch_shortcut(seq)
            }
        }
    };
}

pub trait TuiTab: super::TuiWidget {
    fn title(&self) -> &'static str;
    fn shortcuts(&self) -> &[(KeyCombo, &'static str)];
    fn dispatch_shortcut(&mut self, seq: &[crate::tui::Key]);
}

pub(crate) mod connections;
pub(crate) mod files;
pub(crate) mod logs;
pub(crate) mod proxies;
pub(crate) mod settings;
pub(crate) mod srvctl;
mod status;

macro_rules! enum_dispatch {
    ($vis:vis enum $ident:ident {
        $($item:ident,)+
    }) => {
    $vis enum $ident {
        $($item($item),)+
    }

    $(impl From<$item> for Tab {
        fn from(value: $item) -> Self {
            Self::$item(value)
        }
    })+

    impl crate::tui::TuiWidget for Tab {
        fn handle_key_event(&mut self, kv: &crate::tui::Key) {
            match self {
                $(Self::$item(inner) => inner.handle_key_event(kv),)+
            }
        }

        fn render(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
            match self {
                $(Self::$item(inner) => inner.render(f, area),)+
            }
        }

        fn sync(&mut self) {
            match self {
                $(Self::$item(inner) => inner.sync(),)+
            }
        }

        fn on_enter(&mut self) {
            match self {
                $(Self::$item(inner) => inner.on_enter(),)+
            }
        }

        fn on_leave(&mut self) {
            match self {
                $(Self::$item(inner) => inner.on_leave(),)+
            }
        }
    }

    impl TuiTab for Tab {
        fn title(&self) -> &'static str {
            match self {
                $(Self::$item(inner) => inner.title(),)+
            }
        }

        fn shortcuts(&self) -> &[(crate::tui::widget::tab::KeyCombo, &'static str)] {
            match self {
                $(Self::$item(inner) => inner.shortcuts(),)+
            }
        }

        fn dispatch_shortcut(&mut self, seq: &[crate::tui::Key]) {
            match self {
                $(Self::$item(inner) => inner.dispatch_shortcut(seq),)+
            }
        }
    }

    };
}

pub mod prelude {
    pub use super::TuiTab;
    pub use super::connections::ConnectionsTab;
    pub use super::files::FileTab;
    pub use super::logs::LogsTab;
    pub use super::proxies::ProxiesTab;
    pub use super::settings::SettingsTab;
    pub use super::srvctl::CoreSrvCtlTab;
    pub use super::status::StatusTab;

    pub fn agent_init(keymap: &mut serde_yml::Mapping) -> anyhow::Result<()> {
        use anyhow::Context;

        // Helper: dispatch Mapping (old) vs Sequence (new list-format)
        macro_rules! init_section {
            ($keymap:expr, $section:literal, $tab:ident) => {
                if let Some(section_val) = crate::tui::agent::take_section($keymap, $section) {
                    match section_val {
                        serde_yml::Value::Mapping(map) => {
                            crate::tui::agent::check_duplicate_keys($section, &map);
                            let (keys, descs) = crate::tui::agent::extract_keymap_with_descs(map)?;
                            super::$tab::agent_init(keys);
                            super::$tab::init_descs(descs);
                        }
                        serde_yml::Value::Sequence(seq) => {
                            let entries: Vec<crate::tui::agent::Entry> =
                                serde_yml::from_value(serde_yml::Value::Sequence(seq))
                                    .context(concat!("parsing ", $section, " entries"))?;
                            crate::tui::agent::check_duplicate_keys_list($section, &entries);
                            let (keys, descs, chords) =
                                crate::tui::agent::extract_keymap_list(entries)?;
                            super::$tab::agent_init(keys);
                            super::$tab::init_descs(descs);
                            super::$tab::init_chords(chords);
                        }
                        _ => {
                            anyhow::bail!("Section `{}` is neither Mapping nor Sequence", $section);
                        }
                    }
                }
            };
        }

        init_section!(keymap, "connections", connections);
        init_section!(keymap, "proxies", proxies);
        init_section!(keymap, "srvctl", srvctl);
        init_section!(keymap, "settings", settings);
        init_section!(keymap, "logs", logs);

        // FileTab has nested sections — delegate to files::agent_init
        if let Some(section_val) = crate::tui::agent::take_section(keymap, "file") {
            match section_val {
                serde_yml::Value::Mapping(map) => {
                    super::files::agent_init(map).context("Loading FileTab KeyMap")?;
                }
                _ => {
                    anyhow::bail!(
                        "`file` section only supports Mapping format (nested profile/template)"
                    );
                }
            }
        }

        Ok(())
    }

    enum_dispatch!(
        pub enum Tab {
            ConnectionsTab,
            FileTab,
            ProxiesTab,
            SettingsTab,
            CoreSrvCtlTab,
            StatusTab,
            LogsTab,
        }
    );
}
