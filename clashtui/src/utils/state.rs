use super::SharedClashTuiUtil;
use api::{Mode, TunStack};

pub struct _State {
    pub profile: String,
    pub mode: Option<Mode>,
    pub tun: Option<TunStack>,
    pub sysproxy: Option<bool>,
}
pub struct State {
    st: _State,
    ct: SharedClashTuiUtil,
}
impl State {
    pub fn new(ct: SharedClashTuiUtil) -> Self {
        return Self {
            st: ct.update_state(None, None, None),
            ct,
        };
    }
    pub fn refresh(&mut self){
        self.st = self.ct.update_state(None, None)
    }
    pub fn get_profile(&self) -> &String {
        &self.st.profile
    }
    pub fn set_profile(&mut self, profile: String) {
        // With update state
        self.st = self.ct.update_state(Some(profile), None, None)
    }
    pub fn set_mode(&mut self, mode: String) {
        self.st = self.ct.update_state(None, Some(mode), None)
    }
    pub fn render(&self) -> String {
        let status_str = format!(
            "Profile: {}    Mode: {}    SysProxy: {}    Tun: {}    Help: ?",
            self.st.profile,
            self.st
                .mode
                .as_ref()
                .map_or("Unknown".to_string(), |v| format!("{}", v)),
            self.st
                .sysproxy
                .map_or("Unknown".to_string(), |v| format!("{}", if v {"On"} else {"Off"})),
            self.st
                .tun
                .as_ref()
                .map_or("Unknown".to_string(), |v| format!("{}", v)),
        );
        status_str
    }

    pub fn get_sysproxy(&self) -> Option<bool> {
        self.st.sysproxy
    }

    pub fn set_sysproxy(&mut self, sysproxy: bool) {
        self.st = self.ct.update_state(None, None, Some(sysproxy));
    }
}
