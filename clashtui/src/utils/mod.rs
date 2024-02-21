mod config;
mod flags;
mod ipc;
mod state;
mod tui;
mod tui_impl;
mod utils;

pub type SharedClashTuiUtil = std::rc::Rc<tui::ClashTuiUtil>;
pub type SharedClashTuiState = std::rc::Rc<std::cell::RefCell<State>>;

pub use api::Mode;
pub use config::{init_config, CfgError, ErrKind};
pub use flags::{Flag, Flags};
pub use state::State;
pub use tui::ClashTuiUtil;
pub use utils::*;

macro_rules! define_enum {
    ($name: ident, [$($variant:ident),*]) => {
        #[derive(Debug, PartialEq, Eq, Clone)]
        pub enum $name {
            $($variant),*
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                match value {
                    $(stringify!($variant) => $name::$variant,)*
                    _ => panic!("Invalid value for conversion"),
                }
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                match value {
                    $($name::$variant => String::from(stringify!($variant)),)*
                }
            }
        }
    };
}

#[cfg(target_os = "linux")]
define_enum!(
    ClashSrvOp,
    [
        StartClashService,
        StopClashService,
        TestClashConfig,
        SetPermission,
        SwitchMode
    ]
);

#[cfg(target_os = "windows")]
define_enum!(
    ClashSrvOp,
    [
        StartClashService,
        StopClashService,
        TestClashConfig,
        SwitchSysProxy,
        EnableLoopback,
        InstallSrv,
        UnInstallSrv,
        SwitchMode
    ]
);
