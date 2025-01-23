use crate::clash::webapi::{Mode, TunStack};

pub struct State {
    pub profile: String,
    pub mode: Option<Mode>,
    pub tun: Option<TunStack>,
    #[cfg(target_os = "windows")]
    pub sysproxy: Option<bool>,
}
impl State {
    pub fn unknown(profile: String) -> Self {
        Self {
            profile,
            mode: None,
            tun: None,
            #[cfg(target_os = "windows")]
            sysproxy: None,
        }
    }
}
impl core::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(target_os = "windows")]
        let status_str = write!(
            f,
            "Profile: {}    Mode: {}    SysProxy: {}    Tun: {}    Help: ?",
            self.profile,
            self.mode
                .as_ref()
                .map_or("Unknown".to_string(), |v| format!("{}", v)),
            self.sysproxy
                .map_or("Unknown".to_string(), |v| format!("{}", v)),
            self.tun
                .as_ref()
                .map_or("Unknown".to_string(), |v| format!("{}", v)),
        );
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        let status_str = write!(
            f,
            "Profile: {}    Mode: {}    Tun: {}    Help: ?",
            self.profile,
            self.mode
                .as_ref()
                .map_or("Unknown".to_string(), |v| format!("{}", v)),
            self.tun
                .as_ref()
                .map_or("Unknown".to_string(), |v| format!("{}", v)),
        );
        status_str
    }
}
