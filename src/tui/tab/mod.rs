mod dev {
    pub use crate::tui::widget::dualtab::*;
    pub use crate::tui::widget::tab::*;
    pub use crate::tui::Key as TuiKey;
    pub use crossterm::event::KeyCode;
    pub use ratatui::prelude::{Frame, Rect};
    pub use ratatui::style::{Color, Stylize as _};
    pub use ratatui::widgets::{Block, List, ListState, StatefulWidget};

    pub use crate::tui::popmsg::prelude::*;
    pub use crate::tui::theme::Theme;
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
mod agent {
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

    static SHORTCUTS: OnceLock<Vec<(KeyCombo, $ident, &'static str)>> = OnceLock::new();

    pub fn all_shortcuts() -> &'static [(KeyCombo, $ident, &'static str)] {
        SHORTCUTS.get_or_init(|| {
            let mut v = Vec::new();
            mod_agent!(@shortcuts v, $($tokens)*);
            v
        })
    }
}

use agent::{agent, all_shortcuts};
pub use agent::{init as agent_init};
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

mod connections;
mod files;
mod proxies;
mod settings;
mod srvctl;
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
    pub use super::proxies::ProxiesTab;
    pub use super::settings::SettingsTab;
    pub use super::srvctl::SrvCtlTab;
    pub use super::status::StatusTab;

    pub fn agent_init(keymap: &mut serde_yml::Mapping) -> anyhow::Result<()> {
        use anyhow::Context;

        if let Ok(map) = crate::tui::agent::get(keymap, "connections") {
            let keys = serde_yml::from_value(serde_yml::Value::Mapping(map))?;
            super::connections::agent_init(keys);
        }

        if let Ok(map) = crate::tui::agent::get(keymap, "file") {
            super::files::agent_init(map).context("Loading FileTab KeyMap")?;
        }

        if let Ok(map) = crate::tui::agent::get(keymap, "srvctl") {
            let keys = serde_yml::from_value(serde_yml::Value::Mapping(map))?;
            super::srvctl::agent_init(keys);
        }

        if let Ok(map) = crate::tui::agent::get(keymap, "settings") {
            let keys = serde_yml::from_value(serde_yml::Value::Mapping(map))?;
            super::settings::agent_init(keys);
        }

        Ok(())
    }

    enum_dispatch!(
        pub enum Tab {
            ConnectionsTab,
            FileTab,
            ProxiesTab,
            SettingsTab,
            SrvCtlTab,
            StatusTab,
        }
    );
}
