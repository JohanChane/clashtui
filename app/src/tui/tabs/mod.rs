mod clashsrvctl;
mod config;
mod profile;
mod profile_input;

pub use clashsrvctl::ClashSrvCtlTab;
pub use config::ConfigTab;
pub use profile::ProfileTab;

pub enum Tabs {
    Profile(std::cell::RefCell<ProfileTab>),
    ClashSrvCtl(std::cell::RefCell<ClashSrvCtlTab>),
    Config(std::cell::RefCell<ConfigTab>),
}
#[derive(Eq, Hash, PartialEq)]
pub enum Tab {
    Profile,
    ClashSrvCtl,
    Config,
}
impl std::cmp::PartialEq<std::string::String> for Tab {
    fn eq(&self, other: &std::string::String) -> bool {
        use super::utils::symbols;
        let fmtd = match self {
            Tab::Profile => symbols::PROFILE.to_string(),
            Tab::ClashSrvCtl => symbols::CLASHSRVCTL.to_string(),
            Tab::Config => symbols::CONFIG.to_string(),
        };
        &fmtd == other
    }
}
