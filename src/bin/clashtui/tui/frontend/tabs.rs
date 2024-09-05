#[cfg(feature = "connection-tab")]
pub mod connection;
pub mod profile;
pub mod service;
#[macro_use]
mod misc;

use crate::{
    tui::{frontend::consts, Call, Drawable, EventState, PopMsg},
    utils::CallBack,
};
use crossterm::event::KeyEvent;
use ratatui::prelude as Ra;
use Ra::{Frame, Rect};

#[cfg(feature = "connection-tab")]
use connection::ConnctionTab;
use profile::ProfileTab;
use service::ServiceTab;

/// A trait that every tab should impl
pub(super) trait TabCont {
    fn get_backend_call(&mut self) -> Option<Call>;
    /// return [`PopMsg`], guide the [`super::ConfirmPopup`] to ask
    fn get_popup_content(&mut self) -> Option<PopMsg>;
    fn apply_backend_call(&mut self, op: CallBack);
    /// return [`EventState::WorkDone`] only when the msg popup should close
    fn apply_popup_result(&mut self, evst: EventState) -> EventState;
}

build_tabs!(
    enum Tabs {
        Profile(ProfileTab),
        Service(ServiceTab),
        #[cfg(feature = "connection-tab")]
        Connection(ConnctionTab),
    }
);
/// a wrapper for [`Tabs`]
pub(super) struct TabContainer(Tabs);

impl std::fmt::Display for TabContainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self.0 {
                Tabs::Profile(_) => consts::TAB_TITLE_PROFILE,
                Tabs::Service(_) => consts::TAB_TITLE_SERVICE,
                #[cfg(feature = "connection-tab")]
                Tabs::Connection(_) => consts::TAB_TITLE_CONNECTION,
            }
        )
    }
}
