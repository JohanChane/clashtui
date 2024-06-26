pub(crate) mod config;
#[cfg_attr(target_os = "windows", path = "ipc_windows.rs")]
#[cfg_attr(target_os = "linux", path = "ipc_linux.rs")]
pub(crate) mod ipc;
mod state;
#[allow(clippy::module_inception)]
mod utils;

pub use config::init_config;
pub use state::State;
pub use utils::*;
/// a fix for [`MonkeyPatch`]
pub use ipc::spawn;

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
