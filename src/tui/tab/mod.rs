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
    ($(#[$m:meta])* $tab:ident($ty:ident<$inner:ident>), $scope:ident) => {
        $(#[$m])*
        #[derive(Default)]
        pub struct $tab($ty<$inner>);
        crate::new_type_impl_tuiwidget!($tab);
        impl crate::tui::tab::TuiTab for $tab {
            fn title(&self) -> &'static str { $inner::TITLE }
            fn input_scope(&self) -> crate::tui::key::Scope {
                crate::tui::key::Scope::$scope
            }
            fn handle_action(&mut self, action: crate::tui::key::Action) {
                let crate::tui::key::Action::$scope(action) = action else {
                    unreachable!("action routed to the wrong page");
                };
                self.0.handle_action(action);
            }
        }
    };
    (@no_key $(#[$m:meta])* $tab:ident($ty:ident<$inner:ident>)) => {
        $(#[$m])*
        #[derive(Default)]
        pub struct $tab($ty<$inner>);
        crate::new_type_impl_tuiwidget!($tab);
        impl crate::tui::tab::TuiTab for $tab {
            fn title(&self) -> &'static str { $inner::TITLE }
            fn input_scope(&self) -> crate::tui::key::Scope { crate::tui::key::Scope::App }
            fn handle_action(&mut self, _: crate::tui::key::Action) {
                unreachable!("this page has no local actions");
            }
        }
    };
}

pub(in crate::tui) trait TuiTab: super::TuiWidget {
    fn title(&self) -> &'static str;
    fn input_scope(&self) -> crate::tui::key::Scope;
    fn handle_action(&mut self, action: crate::tui::key::Action);
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

        fn input_scope(&self) -> crate::tui::key::Scope {
            match self {
                $(Self::$item(inner) => inner.input_scope(),)+
            }
        }
        fn handle_action(&mut self, action: crate::tui::key::Action) {
            match self {
                $(Self::$item(inner) => inner.handle_action(action),)+
            }
        }
    }

    };
}

pub mod prelude {
    pub(in crate::tui) use super::TuiTab;
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
