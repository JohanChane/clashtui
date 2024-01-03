pub mod clashsrvctl_tab;
pub mod confirm_popup;
pub mod msgpopup;
pub mod profile_input;
pub mod profile_tab;
pub mod statusbar;
pub mod symbols;
pub mod widgets;

pub use self::clashsrvctl_tab::ClashSrvCtlTab;
pub use self::confirm_popup::ConfirmPopup;
pub use self::msgpopup::MsgPopup;
pub use self::profile_input::ProfileInputPopup;
pub use self::profile_tab::ProfileTab;
pub use self::statusbar::ClashTuiStatusBar;
pub use self::symbols::{SharedSymbols, Symbols};

#[derive(PartialEq, Eq, Clone)]
pub enum EventState {
    NotConsumed,
    WorkDone,
    ProfileUpdate,
    ProfileUpdateAll,
    ProfileSelect,
    ProfileDelete,
    EnableTun,
    DisableTun,
    EnableSysProxy,
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
    TestClashConfig,
    EnableTun,
    DisableTun
);

#[cfg(target_os = "windows")]
define_clashtui_operations!(
    StartClash,
    StopClash,
    TestClashConfig,
    EnableTun,
    DisableTun,
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
