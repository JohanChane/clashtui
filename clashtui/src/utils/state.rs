use super::SharedClashTuiUtil;
use api::{Mode, TunStack};

pub struct _State {
    pub profile: String,
    pub mode: Option<Mode>,
    pub tun: Option<TunStack>,
    #[cfg(target_os = "windows")]
    pub sysproxy: Option<bool>,
    pub ver: String,
}
impl _State {
    #[cfg(target_os = "linux")]
    pub fn new(pf: String, mode: Option<Mode>, tun: Option<TunStack>, ver: String) -> Self {
        Self {
            tun,
            mode,
            profile: pf,
            ver,
        }
    }
    #[cfg(target_os = "windows")]
    pub fn new(
        profile: String,
        mode: Option<Mode>,
        tun: Option<TunStack>,
        ver: String,
        sysproxy: Option<bool>,
    ) -> Self {
        Self {
            profile,
            mode,
            tun,
            sysproxy,
            ver,
        }
    }
}
pub struct State {
    st: _State,
    ct: SharedClashTuiUtil,
}
impl State {
    pub fn new(ct: SharedClashTuiUtil) -> Self {
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
            self.st = self.ct.update_state(Some(mode), None)
        }
    }
    pub fn render(&self) -> String {
        #[cfg(target_os = "windows")]
        let status_str = format!(
            "Profile: {}    Mode: {}    SysProxy: {}    Tun: {}    ClashVer: {}    Help: ?",
            self.st.profile,
            self.st
                .mode
                .as_ref()
                .map_or("Unknown".to_string(), |v| format!("{}", v)),
            self.st
                .sysproxy
                .map_or("Unknown".to_string(), |v| format!("{}", v)),
            self.st
                .tun
                .as_ref()
                .map_or("Unknown".to_string(), |v| format!("{}", v)),
            self.st.ver
        );
        #[cfg(target_os = "linux")]
        let status_str = format!(
            "Profile: {}    Mode: {}    Tun: {}    ClashVer: {}    Help: ?",
            self.st.profile,
            self.st
                .mode
                .as_ref()
                .map_or("Unknown".to_string(), |v| format!("{}", v)),
            self.st
                .tun
                .as_ref()
                .map_or("Unknown".to_string(), |v| format!("{}", v)),
            self.st.ver
        );
        status_str
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
