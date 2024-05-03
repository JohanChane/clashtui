mod backend;
mod config;
mod ipc;
mod state;
#[allow(clippy::module_inception)]
mod utils;

pub use backend::ClashBackend;
pub use config::{init_config, CfgError};
pub use state::State;
pub use utils::*;

#[macro_export]
macro_rules! define_enum {
    ($(#[$attr:meta])*
    $vis:vis $name: ident,
    [$($variant:ident),*]) => {
        $(#[$attr])*
        $vis enum $name {
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
    pub ClashSrvOp,
    [
        StartClashService,
        StopClashService,
        SetPermission,
        SwitchMode
    ]
);
#[cfg(target_os = "windows")]
define_enum!(
    pub ClashSrvOp,
    [
        StartClashService,
        StopClashService,
        SwitchSysProxy,
        EnableLoopback,
        InstallSrv,
        UnInstallSrv,
        SwitchMode
    ]
);
