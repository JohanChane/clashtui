mod dev {
    pub use crate::tui::widget::dualtab::*;
    pub use crate::tui::widget::tab::*;
    pub use crossterm::event::{KeyCode, KeyEvent};
    pub use ratatui::prelude::{Frame, Rect};
    pub use ratatui::style::Stylize as _;
    pub use ratatui::widgets::{Block, List, ListState, StatefulWidget};

    pub use crate::tui::popmsg::prelude::*;
    pub use crate::tui::theme::Theme;
}

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
}

macro_rules! mod_agent {
    ($ident: ident, [$(($raw:expr,$map:expr),)+]) => {
mod agent {
    use super::*;
    use crossterm::event::*;
    use std::collections::HashMap;
    use std::sync::OnceLock;

    pub type Agent = HashMap<KeyEvent, $ident>;

    static AGENT: OnceLock<Agent> = OnceLock::new();

    fn default_agent() -> Agent {
        fn quick_map(code: KeyCode) -> KeyEvent {
            KeyEvent::new_with_kind_and_state(
                code,
                KeyModifiers::empty(),
                KeyEventKind::Press,
                KeyEventState::empty(),
            )
        }
        [
            $(($raw,$map),)+
        ]
        .into_iter()
        .map(|(code, key)| (quick_map(code), key))
        .collect()
    }

    pub fn agent() -> &'static Agent{
        AGENT.get_or_init(default_agent)
    }

    pub fn init(map: Agent) {
        if AGENT.set(map).is_err() {
            unreachable!("KeyMap Init Twice!")
        }
    }
}

use agent::{agent};
pub use agent::{init as agent_init};
    };
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
        }
    };
}

pub trait TuiTab: super::TuiWidget {
    fn title(&self) -> &'static str;
}

mod files;
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
        fn handle_key_event(&mut self, kv: &crossterm::event::KeyEvent) {
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
    }

    };
}

pub mod prelude {
    pub use super::TuiTab;
    pub use super::files::FileTab;
    pub use super::status::StatusTab;

    pub fn agent_init(keymap: &mut serde_yml::Mapping) -> anyhow::Result<()> {
        use anyhow::Context;

        if let Ok(map) = crate::tui::agent::get(keymap, "file") {
            super::files::agent_init(map).context("Loading FileTab KeyMap")?;
        }

        Ok(())
    }

    enum_dispatch!(
        pub enum Tab {
            FileTab,
            StatusTab,
        }
    );
}
