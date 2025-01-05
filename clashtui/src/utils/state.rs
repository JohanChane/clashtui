use super::SharedClashTuiUtil;
use api::{Mode, TunStack};

pub struct _State {
    pub profile: String,
    pub mode: Option<Mode>,
    pub tun: Option<TunStack>,
    pub no_pp: bool                 // no proxy providers
}
pub struct State {
    st: _State,
    ct: SharedClashTuiUtil,
}
impl State {
    pub fn new(ct: SharedClashTuiUtil) -> Self {
        Self {
            st: ct.update_state(None, None, None),
            ct,
        }
    }
    pub fn refresh(&mut self){
        self.st = self.ct.update_state(None, None, None)
    }
    pub fn get_profile(&self) -> &String {
        &self.st.profile
    }
    pub fn set_profile(&mut self, profile: String) {
        self.st = self.ct.update_state(Some(profile), None, None)
    }
    pub fn set_mode(&mut self, mode: String) {
        self.st = self.ct.update_state(None, Some(mode), None)
    }
    pub fn switch_no_pp(&mut self) {
        let no_pp = !self.st.no_pp;
        self.st = self.ct.update_state(None, None, Some(no_pp))
    }
    pub fn get_no_pp(&self) -> bool {
        self.st.no_pp
    }
    pub fn render(&self) -> String {
        let status_str = format!(
            "Profile: {}    Mode: {}    Tun: {}    NoPp: {}    Help: ?",
            self.st.profile,
            self.st
                .mode
                .as_ref()
                .map_or("Unknown".to_string(), |v| format!("{}", v)),
            self.st
                .tun
                .as_ref()
                .map_or("Unknown".to_string(), |v| format!("{}", v)),
            if self.st.no_pp {"On"} else {"Off"},
        );
        status_str
    }
}
