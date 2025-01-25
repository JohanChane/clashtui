#[cfg(feature = "connection-tab")]
pub mod connection;
pub mod profile;
pub mod service;

use crate::{
    tui::{widget::PopRes, Call, Drawable, EventState, PopMsg},
    utils::CallBack,
};
/// A trait that every tab should impl
pub(super) trait TabCont: Drawable + std::fmt::Display {
    fn get_backend_call(&mut self) -> Option<Call>;
    /// return [`PopMsg`], guide the [`super::ConfirmPopup`] to ask
    fn get_popup_content(&mut self) -> Option<PopMsg>;
    fn apply_backend_call(&mut self, op: CallBack);
    /// return [`EventState::WorkDone`] only when the msg popup should close
    fn apply_popup_result(&mut self, res: PopRes) -> EventState;
}
