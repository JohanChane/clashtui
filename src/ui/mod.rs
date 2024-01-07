mod confirm_popup;
mod statusbar;
mod tabbar;
pub mod utils;
pub mod popups;
pub mod tabs;
pub mod keys;

pub use self::confirm_popup::ConfirmPopup;
pub use self::statusbar::ClashTuiStatusBar;
pub use self::keys::symbols::{SharedSymbols, Symbols};

pub use tabbar::ClashTuiTabBar;

#[derive(PartialEq, Eq, Clone)]
pub enum EventState {
    UnexpectedERROR,
    NotConsumed,
    WorkDone,
    ProfileUpdate,
    ProfileUpdateAll,
    ProfileSelect,
    ProfileDelete,
    #[cfg(target_os = "windows")]
    EnableSysProxy,
    #[cfg(target_os = "windows")]
    DisableSysProxy,
}

impl EventState {
    pub fn is_consumed(&self) -> bool {
        !self.is_notconsumed()
    }
    pub fn is_notconsumed(&self) -> bool {
        *self == Self::NotConsumed
    }
}


#[macro_export]
macro_rules! msgpopup_methods {
    ($type:ident) => {
        impl $type {
            pub fn popup_txt_msg(&mut self, msg: String) {
                self.msgpopup.push_txt_msg(msg);
                self.msgpopup.show();
            }
            pub fn popup_list_msg(&mut self, msg: Vec<String>) {
                self.msgpopup.push_list_msg(msg);
                self.msgpopup.show();
            }
            pub fn hide_msgpopup(&mut self) {
                self.msgpopup.hide();
            }
        }
    };
}
