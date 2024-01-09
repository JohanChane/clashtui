mod clash;
mod clash_state;
mod clashtui;
mod configs;
pub mod utils;

pub use self::clash_state::State;
pub use self::clashtui::ClashTuiUtil;
pub use self::configs::{init_config, ClashTuiConfigLoadError};

pub type SharedClashTuiUtil = std::rc::Rc<ClashTuiUtil>;
pub type SharedClashTuiState = std::rc::Rc<std::cell::RefCell<State>>;

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
define_clashtui_operations!(StartClash, StopClash, TestClashConfig);

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
