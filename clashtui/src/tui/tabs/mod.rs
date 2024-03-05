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
pub trait TabEvent {
    fn draw(&mut self, f: &mut ratatui::prelude::Frame, area: ratatui::prelude::Rect);
    fn popup_event(
        &mut self,
        ev: &crossterm::event::Event,
    ) -> Result<ui::EventState, impl std::error::Error>;
    fn event(
        &mut self,
        ev: &crossterm::event::Event,
    ) -> Result<ui::EventState, impl std::error::Error>;
    fn late_event(&mut self);
}

#[macro_export]
macro_rules! msgpopup_methods {
    ($type:ident) => {
        impl $type {
            pub fn popup_txt_msg(&mut self, msg: String) {
                self.msgpopup.push_txt_msg(msg);
                self.msgpopup.show();
            }
            pub fn popup_list_msg(&mut self, msg: impl IntoIterator<Item = String>) {
                self.msgpopup.push_list_msg(msg);
                self.msgpopup.show();
            }
            pub fn hide_msgpopup(&mut self) {
                self.msgpopup.hide();
            }
        }
    };
}
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
    ClashSrvOp,
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
