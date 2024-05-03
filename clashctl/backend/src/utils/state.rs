use api::{Mode, TunStack};

pub struct State {
    pub profile: String,
    pub mode: Option<Mode>,
    pub tun: Option<TunStack>,
    #[cfg(target_os = "windows")]
    pub sysproxy: Option<bool>,
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
        #[cfg(target_os = "linux")]
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
