mod config;
mod flags;
mod ipc;
mod state;
mod tui;
mod tui_impl;
pub mod utils;

pub type SharedClashTuiUtil = std::rc::Rc<tui::ClashTuiUtil>;
pub type SharedClashTuiState = std::rc::Rc<std::cell::RefCell<State>>;

pub use config::{init_config, ClashTuiConfigLoadError};
pub use flags::{Flag,Flags};
pub(self) use ipc::exec_ipc;
pub use state::State;
pub use tui::ClashTuiUtil;

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

        impl Into<String> for $name {
            fn into(self) -> String {
                match self {
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
        EnableSysProxy,
        DisableSysProxy,
        EnableLoopback,
        InstallSrv,
        UnInstallSrv
    ]
);

define_enum!(
    CfgOp,
    [
        ClashConfigDir,
        ClashCorePath,
        ClashConfigFile,
        ClashServiceName,
        TuiEdit,
        TuiOpen
    ]
);
