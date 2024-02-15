mod statusbar;
mod tabbar;
pub mod tabs;
pub mod utils;
pub mod widgets;

pub use statusbar::StatusBar;
pub use tabbar::TabBar;

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
        *self == Self::NotConsumed
    }
}
