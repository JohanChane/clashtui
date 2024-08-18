#![allow(refining_impl_trait)]
mod clashsrvctl;
mod profile;
mod profile_input;

pub use clashsrvctl::ClashSrvCtlTab;
pub use profile::ProfileTab;

use crate::ui;

pub enum Tabs {
    Profile(ProfileTab),
    ClashSrvCtl(ClashSrvCtlTab),
}
impl std::fmt::Display for Tabs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::tui::symbols;
        write!(
            f,
            "{}",
            match self {
                Tabs::Profile(_) => symbols::PROFILE.to_string(),
                Tabs::ClashSrvCtl(_) => symbols::CLASHSRVCTL.to_string(),
            }
        )
    }
}
impl std::cmp::PartialEq<std::string::String> for Tabs {
    fn eq(&self, other: &std::string::String) -> bool {
        let fmtd = self.to_string();
        &fmtd == other
    }
}
pub trait TabEvent {
    fn draw(&mut self, f: &mut ratatui::prelude::Frame, area: ratatui::prelude::Rect);
    fn popup_event(
        &mut self,
        ev: &ui::event::Event,
    ) -> Result<ui::EventState, impl std::error::Error>;
    fn event(&mut self, ev: &ui::event::Event) -> Result<ui::EventState, impl std::error::Error>;
    fn late_event(&mut self);
}

#[macro_export]
macro_rules! msgpopup_methods {
    ($type:ident) => {
        impl $type {
            // single-line popup
            pub fn popup_txt_msg(&mut self, msg: String) {
                self.msgpopup.push_txt_msg(msg);
                self.msgpopup.show();
            }
            // multi-lines popup
            pub fn popup_list_msg<I>(&mut self, msg: I)
            where
                I: IntoIterator<Item = String>,
            {
                self.msgpopup.push_list_msg(msg);
                self.msgpopup.show();
            }
            #[allow(unused)]
            pub fn hide_msgpopup(&mut self) {
                self.msgpopup.hide();
            }
        }
    };
}
