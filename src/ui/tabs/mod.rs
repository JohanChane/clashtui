mod clashsrvctl_tab;
mod config_tab;
mod profile_input;
mod profile_tab;

pub use clashsrvctl_tab::ClashSrvCtlTab;
pub use config_tab::ConfigTab;
pub use profile_tab::ProfileTab;

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
