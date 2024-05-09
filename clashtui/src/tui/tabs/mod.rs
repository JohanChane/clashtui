mod clashsrvctl;
mod profile;
mod profile_input;

pub use clashsrvctl::ClashSrvCtlTab;
pub use profile::ProfileTab;

pub enum Tabs {
    Profile(ProfileTab),
    ClashSrvCtl(ClashSrvCtlTab),
}
impl ToString for Tabs {
    fn to_string(&self) -> String {
        use super::symbols;
        match self {
            Tabs::Profile(_) => symbols::PROFILE.to_string(),
            Tabs::ClashSrvCtl(_) => symbols::CLASHSRVCTL.to_string(),
        }
    }
}
impl std::cmp::PartialEq<std::string::String> for Tabs {
    fn eq(&self, other: &std::string::String) -> bool {
        let fmtd = self.to_string();
        &fmtd == other
    }
}
pub trait TabEvent {
    fn draw(&mut self, f: &mut ratatui::prelude::Frame, area: ratatui::prelude::Rect);
    fn popup_event(
        &mut self,
        ev: &ui::event::Event,
    ) -> Result<ui::EventState, impl std::error::Error>;
    fn event(
        &mut self,
        ev: &ui::event::Event,
    ) -> Result<ui::EventState, impl std::error::Error>;
    fn late_event(&mut self);
}

#[macro_export]
macro_rules! msgpopup_methods {
    ($type:ident) => {
        impl $type {
            // single-line popup
            pub fn popup_txt_msg(&mut self, msg: String) {
                if ! msg.is_empty() {
                    self.msgpopup.push_txt_msg(msg);
                    self.msgpopup.show();
                }
            }
            // multi-lines popup
            pub fn popup_list_msg<I>(&mut self, msg: I)
            where
                I: IntoIterator<Item = String>,
            {
                let mut list_msg = Vec::<String>::new();
                for m in msg.into_iter() {
                    list_msg.push(m);
                }
                if list_msg.len() > 0 {
                    self.msgpopup.push_list_msg(list_msg);
                    self.msgpopup.show();
                }
            }
            #[allow(unused)]
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

define_enum!(
    #[derive(Clone)]
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
