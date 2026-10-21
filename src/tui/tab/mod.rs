mod dev {
    pub use crate::tui::key::Document;
    pub use crate::tui::key::instancing::defaults::DefaultMap;
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
        newtype_tab!($(#[$m])* $tab($ty<$inner>), $inner::TITLE, km::get_docs(), km::pending_keys(), km::cancel_pending(), km::has_binding, km::starts_chord);
    };
    (@no_key $(#[$m:meta])* $tab:ident($ty:ident<$inner:ident>)) => {
        newtype_tab!($(#[$m])* $tab($ty<$inner>), $inner::TITLE, vec![], None, (), |_| false, |_| false);
    };
    ($(#[$m:meta])* $tab:ident($inner:ty), $title:expr, $key_desc:expr, $pending:expr, $cancel:expr, $has_binding:expr, $starts_chord:expr) => {
        $(#[$m])*
        #[derive(Default)]
        pub struct $tab($inner);

        crate::new_type_impl_tuiwidget!($tab);

        impl crate::tui::tab::TuiTab for $tab {
            fn title(&self) -> &'static str {
                $title
            }

            fn key_description(&self) -> crate::tui::key::KeyDesc {
                $key_desc
            }

            fn pending_keys(&self) -> Option<&'static crate::tui::key::WhichHints> {
                $pending
            }

            fn cancel_pending(&self) {
                $cancel
            }

            fn has_binding(&self, key: &crate::tui::Key) -> bool {
                ($has_binding)(key)
            }

            fn starts_chord(&self, key: &crate::tui::Key) -> bool {
                ($starts_chord)(key)
            }
        }
    };
}

pub trait TuiTab: super::TuiWidget {
    fn title(&self) -> &'static str;
    fn key_description(&self) -> crate::tui::key::KeyDesc;
    fn pending_keys(&self) -> Option<&'static crate::tui::key::WhichHints>;
    fn cancel_pending(&self);
    fn has_binding(&self, key: &crate::tui::Key) -> bool;
    fn starts_chord(&self, key: &crate::tui::Key) -> bool;
}

pub(super) mod connections;
pub(super) mod files;
pub(super) mod logs;
pub(super) mod proxies;
pub(super) mod settings;
pub(super) mod srvctl;
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

        fn key_description(&self) -> crate::tui::key::KeyDesc {
            match self {
                $(Self::$item(inner) => inner.key_description(),)+
            }
        }

        fn pending_keys(&self) -> Option<&'static crate::tui::key::WhichHints> {
            match self {
                $(Self::$item(inner) => inner.pending_keys(),)+
            }
        }

        fn cancel_pending(&self) {
            match self {
                $(Self::$item(inner) => inner.cancel_pending(),)+
            }
        }

        fn has_binding(&self, key: &crate::tui::Key) -> bool {
            match self {
                $(Self::$item(inner) => inner.has_binding(key),)+
            }
        }

        fn starts_chord(&self, key: &crate::tui::Key) -> bool {
            match self {
                $(Self::$item(inner) => inner.starts_chord(key),)+
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
