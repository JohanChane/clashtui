mod error;
mod theme;
pub mod tools;
pub use error::Infailable;
pub use theme::Theme;

#[derive(PartialEq, Eq)]
pub enum EventState {
    Yes,
    Cancel,
    NotConsumed,
    WorkDone,
    ProfileUpdate,
    ProfileUpdateAll,
    ProfileSelect,
    ProfileDelete,
    #[cfg(target_os = "windows")]
    SwitchSysProxy,
}

impl EventState {
    pub fn is_consumed(&self) -> bool {
        !self.is_notconsumed()
    }
    pub fn is_notconsumed(&self) -> bool {
        self == &Self::NotConsumed
    }
}
