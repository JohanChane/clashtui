use super::SharedClashBackend;
use api::{Mode, TunStack};

pub struct _State {
    pub profile: String,
    pub mode: Option<Mode>,
    pub tun: Option<TunStack>,
    #[cfg(target_os = "windows")]
    pub sysproxy: Option<bool>,
}
impl core::fmt::Display for _State {
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
pub struct State {
    st: _State,
    ct: SharedClashBackend,
}
impl State {
    pub fn new(ct: SharedClashBackend) -> Self {
        #[cfg(target_os = "windows")]
        return Self {
            st: ct.update_state(None, None, None),
            ct,
        };
        #[cfg(target_os = "linux")]
        Self {
            st: ct.update_state(None, None),
            ct,
        }
    }
    pub fn get_profile(&self) -> &String {
        &self.st.profile
    }
    pub fn set_profile(&mut self, profile: String) {
        // With update state
        #[cfg(target_os = "windows")]
        {
            self.st = self.ct.update_state(Some(profile), None, None)
        }
        #[cfg(target_os = "linux")]
        {
            self.st = self.ct.update_state(Some(profile), None)
        }
    }
    pub fn set_mode(&mut self, mode: String) {
        #[cfg(target_os = "windows")]
        {
            self.st = self.ct.update_state(None, Some(mode), None)
        }
        #[cfg(target_os = "linux")]
        {
            self.st = self.ct.update_state(None, Some(mode))
        }
    }
    pub fn render(&self) -> String {
        self.st.to_string()
    }
    #[cfg(target_os = "windows")]
    pub fn get_sysproxy(&self) -> Option<bool> {
        self.st.sysproxy
    }
    #[cfg(target_os = "windows")]
    pub fn set_sysproxy(&mut self, sysproxy: bool) {
        self.st = self.ct.update_state(None, None, Some(sysproxy));
    }
}
