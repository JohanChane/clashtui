mod dev {
    // pub use crate::tui::key::Key as TuiKey;
    pub use crate::tui::widget::dualtab::*;
    pub use crate::tui::widget::tab::*;
    pub use crossterm::event::KeyCode;
    pub use ratatui::prelude::{Frame, Rect};
    pub use ratatui::style::{Color, Stylize as _};
    pub use ratatui::widgets::{Block, List, ListState, StatefulWidget};

    pub use crate::tui::popmsg::prelude::*;
    pub(crate) use crate::tui::theme::Theme;
}

macro_rules! tri {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => {
                Confirm::err(e);
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
    #[allow(clippy::large_enum_variant, clippy::enum_variant_names)]
    $vis enum $ident {
        $($item($item),)+
    }

    $(impl From<$item> for Tab {
        fn from(value: $item) -> Self {
            Self::$item(value)
        }
    })+

    impl crate::tui::TuiWidget for Tab {
        fn handle_key_event(&mut self, kv: &crate::tui::key::Key) {
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
