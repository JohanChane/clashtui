pub mod profile;
pub mod service;
#[macro_use]
mod misc;

use crate::{
    tui::{frontend::consts::TAB_TITLE_SERVICE, Call, Drawable, EventState, PopMsg},
    utils::CallBack,
};
use crossterm::event::KeyEvent;
use ratatui::prelude as Ra;
use service::ServiceTab;
use Ra::{Frame, Rect};

pub(super) trait TabCont {
    fn get_backend_call(&mut self) -> Option<Call>;
    /// return [`PopMsg`], guide the [`ConfirmPopup`] to ask
    fn get_popup_content(&mut self) -> Option<PopMsg>;
    fn apply_backend_call(&mut self, op: CallBack);
    fn apply_popup_result(&mut self, evst: EventState) -> EventState;
}

build_tabs!(
    enum Tabs {
        // Profile,
        Service(ServiceTab),
    }
);
pub(super) struct TabContainer(Tabs);

impl std::fmt::Display for TabContainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self.0 {
                // Tabs::Profile => todo!(),
                Tabs::Service(_) => TAB_TITLE_SERVICE.to_string(),
            }
        )
    }
}
