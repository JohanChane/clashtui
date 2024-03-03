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
