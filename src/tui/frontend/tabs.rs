#[cfg(feature = "connections")]
pub mod connection;
pub mod profile;
pub mod service;

use crate::{
    backend::CallBack,
    tui::{Call, Drawable, PopMsg, widget::PopRes},
};
/// A trait that every tab should impl
pub(super) trait TabCont: Drawable + std::fmt::Display + Send {
    fn get_backend_call(&mut self) -> Option<Call>;
    /// return [`PopMsg`], guide the [`super::ConfirmPopup`] to ask
    fn get_popup_content(&mut self) -> Option<PopMsg>;
    fn apply_backend_call(&mut self, op: CallBack);
    fn apply_popup_result(&mut self, res: PopRes);

    fn to_dyn(self) -> Box<dyn TabCont>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}
