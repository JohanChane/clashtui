mod clashsrvctl;
mod profile;
mod profile_input;

pub use clashsrvctl::ClashSrvCtlTab;
pub use profile::ProfileTab;

pub enum Tabs {
    Profile(std::cell::RefCell<ProfileTab>),
    ClashSrvCtl(std::cell::RefCell<ClashSrvCtlTab>),
}
#[derive(Eq, Hash, PartialEq)]
pub enum Tab {
    Profile,
    ClashSrvCtl,
}
impl ToString for Tab {
    fn to_string(&self) -> String {
        use super::symbols;
        match self {
            Tab::Profile => symbols::PROFILE.to_string(),
            Tab::ClashSrvCtl => symbols::CLASHSRVCTL.to_string(),
        }
    }
}
impl std::cmp::PartialEq<std::string::String> for Tab {
    fn eq(&self, other: &std::string::String) -> bool {
        let fmtd = self.to_string();
        &fmtd == other
    }
}

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
        UpdateGeoData,
        SetPermission,
        SwitchMode
    ]
);
define_enum!(
    PTOp,
    [
        ProfileUpdate,
        ProfileUpdateAll,
        ProfileSelect,
        ProfileDelete]
);

#[cfg(target_os = "windows")]
define_enum!(
    ClashSrvOp,
    [
        StartClashService,
        StopClashService,
        TestClashConfig,
        UpdateGeoData,
        SwitchSysProxy,
        EnableLoopback,
        InstallSrv,
        UnInstallSrv,
        SwitchMode
    ]
);