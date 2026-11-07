mod dev {
    pub(in crate::tui) use crate::tui::business::{Business, BusinessView};
    pub(in crate::tui) use crate::tui::key::DefaultBindings;
    pub use crate::tui::key::Document;
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
                Confirm::err_async(e).await;
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
            fn handle_action(&mut self, action: crate::tui::key::Action, cx: &mut crate::tui::business::Business) {
                let crate::tui::key::Action::$scope(action) = action else {
                    unreachable!("action routed to the wrong page");
                };
                self.0.handle_action(action, cx);
            }
        }
    };
    (@no_action $(#[$m:meta])* $tab:ident($ty:ident<$inner:ident>)) => {
        $(#[$m])*
        #[derive(Default)]
        pub struct $tab($ty<$inner>);
        crate::new_type_impl_tuiwidget!($tab);
        impl crate::tui::tab::TuiTab for $tab {
            fn title(&self) -> &'static str { $inner::TITLE }
            fn input_scope(&self) -> crate::tui::key::Scope { crate::tui::key::Scope::App }
            fn handle_action(&mut self, _: crate::tui::key::Action, _cx: &mut crate::tui::business::Business) {
                unreachable!("this page has no local actions");
            }
        }
    };
}

pub(in crate::tui) trait TuiTab: super::TuiWidget {
    fn title(&self) -> &'static str;
    fn input_scope(&self) -> crate::tui::key::Scope;
    fn handle_action(
        &mut self,
        action: crate::tui::key::Action,
        cx: &mut crate::tui::business::Business,
    );
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
        fn render(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect, view: crate::tui::business::BusinessView<'_>) {
            match self {
                $(Self::$item(inner) => inner.render(f, area, view),)+
            }
        }

        fn on_enter(&mut self, business: &mut crate::tui::business::Business) {
            match self { $(Self::$item(inner) => inner.on_enter(business),)+ }
        }
        fn on_leave(&mut self, business: &mut crate::tui::business::Business) {
            match self { $(Self::$item(inner) => inner.on_leave(business),)+ }
        }
        fn sync_from(&mut self, view: crate::tui::business::BusinessView<'_>) {
            match self { $(Self::$item(inner) => inner.sync_from(view),)+ }
        }
        fn advance(&mut self) {
            match self { $(Self::$item(inner) => inner.advance(),)+ }
        }
        fn poll_tasks(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<()> {
            match self { $(Self::$item(inner) => inner.poll_tasks(cx),)+ }
        }
        fn finish_task(&mut self, visible: bool) -> bool {
            match self { $(Self::$item(inner) => inner.finish_task(visible),)+ }
        }
        fn has_tasks(&self) -> bool {
            match self { $(Self::$item(inner) => inner.has_tasks(),)+ }
        }
        fn needs_animation(&self) -> bool {
            match self { $(Self::$item(inner) => inner.needs_animation(),)+ }
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
        fn handle_action(&mut self, action: crate::tui::key::Action, cx: &mut crate::tui::business::Business) {
            match self {
                $(Self::$item(inner) => inner.handle_action(action, cx),)+
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
