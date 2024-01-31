mod clashsrvctl_tab;
mod config_tab;
mod profile_input;
mod profile_tab;

pub use clashsrvctl_tab::ClashSrvCtlTab;
pub use config_tab::ConfigTab;
pub use profile_tab::ProfileTab;

pub enum Tabs {
    ProfileTab(std::cell::RefCell<ProfileTab>),
    ClashSrvCtlTab(std::cell::RefCell<ClashSrvCtlTab>),
    ConfigTab(std::cell::RefCell<ConfigTab>),
}
#[derive(Eq, Hash, PartialEq)]
pub enum Tab {
    ProfileTab,
    ClashSrvCtlTab,
    ConfigTab,
}
impl std::cmp::PartialEq<std::string::String> for Tab {
    fn eq(&self, other: &std::string::String) -> bool {
        use super::utils::symbols;
        let fmtd = match self {
            Tab::ProfileTab => symbols::PROFILE.to_string(),
            Tab::ClashSrvCtlTab => symbols::CLASHSRVCTL.to_string(),
            Tab::ConfigTab => symbols::CONFIG.to_string(),
        };
        &fmtd == other
    }
}
