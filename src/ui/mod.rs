pub mod clashsrvctl_tab;
pub mod confirm_popup;
pub mod msgpopup;
pub mod profile_input;
pub mod profile_tab;
pub mod statusbar;
pub mod widgets;
pub mod keys;

use std::cell::RefCell;

pub use self::clashsrvctl_tab::ClashSrvCtlTab;
pub use self::confirm_popup::ConfirmPopup;
pub use self::msgpopup::MsgPopup;
pub use self::profile_input::ProfileInputPopup;
use self::profile_tab::ProfileTab;
pub use self::statusbar::ClashTuiStatusBar;
pub use self::keys::symbols::{SharedSymbols, Symbols};

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


pub trait CommonTab {
    fn draw<B: ratatui::backend::Backend>(&mut self, f: &mut ratatui::Frame<B>, area: ratatui::layout::Rect);
    // This should be impled, but rustc won't recognize it
    fn event(&mut self, ev: &crossterm::event::Event) -> Result<EventState, ()>;
    // Desprate HashMap<_,Box<dyn CommonTab>>
    // fn as_any(&self) -> &dyn std::any::Any;
    // just return &self
}

pub enum Tabs {
    ProfileTab(RefCell<ProfileTab>),
    ClashsrvctlTab(RefCell<ClashSrvCtlTab>),
}

// impl Display for Tabs {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let val = match self {
//             Tabs::ClashsrvctlTab(_) => "Clashsrvctl",
//             Tabs::ProfileTab(_) => "Profile",
//         };
//         write!(f, "{}", val)
//     }
// }

macro_rules! define_clashtui_operations {
    ($($variant:ident),*) => {
        #[derive(Debug, PartialEq, Eq)]
        pub enum ClashTuiOp {
            $($variant),*
        }

        impl From<&str> for ClashTuiOp {
            fn from(value: &str) -> Self {
                match value {
                    $(stringify!($variant) => ClashTuiOp::$variant,)*
                    _ => panic!("Invalid value for conversion"),
                }
            }
        }

        impl Into<String> for ClashTuiOp {
            fn into(self) -> String {
                match self {
                    $(ClashTuiOp::$variant => String::from(stringify!($variant)),)*
                }
            }
        }
    };
}

#[cfg(target_os = "linux")]
define_clashtui_operations!(
    StartClash,
    StopClash,
    TestClashConfig
);

#[cfg(target_os = "windows")]
define_clashtui_operations!(
    StartClash,
    StopClash,
    TestClashConfig,
    EnableSysProxy,
    DisableSysProxy,
    EnableLoopback,
    InstallSrv,
    UnInstallSrv
);

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
