use super::SharedClashTuiUtil;
use api::{Mode, TunStack};

pub struct _State {
    pub profile: String,
    pub mode: Option<Mode>,
    pub tun: Option<TunStack>,
}
pub struct State {
    st: _State,
    ct: SharedClashTuiUtil,
}
impl State {
    pub fn new(ct: SharedClashTuiUtil) -> Self {
        Self {
            st: ct.update_state(None, None),
            ct,
        }
    }
    pub fn get_profile(&self) -> &String {
        &self.st.profile
    }
    pub fn set_profile(&mut self, profile: String) {
        self.st = self.ct.update_state(Some(profile), None)
    }
    pub fn set_mode(&mut self, mode: String) {
        self.st = self.ct.update_state(None, Some(mode))
    }
    pub fn render(&self) -> String {
        let status_str = format!(
            "Profile: {}    Mode: {}    Tun: {}    Help: ?",
            self.st.profile,
            self.st
                .mode
                .as_ref()
                .map_or("Unknown".to_string(), |v| format!("{}", v)),
            self.st
                .tun
                .as_ref()
                .map_or("Unknown".to_string(), |v| format!("{}", v)),
        );
        status_str
    }
}
