use super::{SharedBackend, _State};
pub struct State {
    st: _State,
    ct: SharedBackend,
}
impl State {
    pub fn new(ct: SharedBackend) -> Self {
        #[cfg(target_os = "windows")]
        return Self {
            st: ct.update_state(None, None, None),
            ct,
        };
    }
    pub fn refresh(&mut self) {
        #[cfg(target_os = "windows")]
        {
            self.st = self.ct.update_state(None, None, None)
        }
        #[cfg(target_os = "linux")]
        {
            self.st = self.ct.update_state(None, None)
        }
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
        self.st.to_string()
    }
}
