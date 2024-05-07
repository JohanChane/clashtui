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
        #[cfg(target_os = "linux")]
        Self {
            st: ct.update_state(None, None),
            ct,
        }
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
