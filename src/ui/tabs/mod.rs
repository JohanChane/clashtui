mod clashsrvctl_tab;
mod config_tab;
mod profile_input;
mod profile_tab;

pub use clashsrvctl_tab::ClashSrvCtlTab;
pub use config_tab::ConfigTab;
pub use profile_tab::ProfileTab;

pub trait CommonTab {
    fn draw(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect);
    // This should be impled, but rustc won't recognize it
    fn event(&mut self, ev: &crossterm::event::Event) -> Result<super::EventState, ()>;
    // Desprate HashMap<_,Box<dyn CommonTab>>
    // fn as_any(&self) -> &dyn std::any::Any;
    // just return &self
}

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