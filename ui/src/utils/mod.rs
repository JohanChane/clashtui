mod theme;
pub mod tools;
pub use theme::Theme;
pub type SharedTheme = std::rc::Rc<Theme>;

#[derive(PartialEq, Eq, Clone)]
pub enum EventState {
    NotConsumed,
    WorkDone,
    ProfileUpdate,
    ProfileUpdateAll,
    ProfileSelect,
    ProfileDelete,
    #[cfg(target_os = "windows")]
    EnableSysProxy,
    #[cfg(target_os = "windows")]
    DisableSysProxy,
}

impl EventState {
    pub fn is_consumed(&self) -> bool {
        !self.is_notconsumed()
    }
    pub fn is_notconsumed(&self) -> bool {
        self == &Self::NotConsumed
    }
}
