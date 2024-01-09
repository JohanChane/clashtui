use super::SharedClashTuiUtil;

#[cfg(target_os = "linux")]
pub struct _State {
    pub profile: String,
    pub tun: String,
}
#[cfg(target_os = "linux")]
impl _State {
    pub fn new(pf: String, tun: String) -> Self {
        Self { tun, profile: pf }
    }
}
#[cfg(target_os = "windows")]
pub struct State {
    pub profile: String,
    pub tun: String,
    pub sysproxy: bool,
}
#[cfg(target_os = "windows")]
impl _State {
    pub fn new(pf: String, tun: String, syp: bool) -> Self {
        Self {
            tun,
            profile: pf,
            sysproxy: syp,
        }
    }
}
pub struct State {
    pub st: _State,
    ct: SharedClashTuiUtil,
}
impl State {
    pub fn new(ct: SharedClashTuiUtil) -> Self {
        Self {
            st: ct.update_state(None),
            ct,
        }
    }
    pub fn get_profile(&self) -> &String {
        &self.st.profile
    }
    pub fn get_tun(&self) -> &String {
        &self.st.tun
    }
    pub fn set_profile(&mut self, profile: String) {
        // With update state
        self.st = self.ct.update_state(Some(profile));
    }
    pub fn update_tun(&mut self) {
        self.st = self.ct.update_state(None);
    }
    #[cfg(target_os = "windows")]
    pub fn get_sysproxy(&self) -> bool {
        self.state.sysproxy
    }
    #[cfg(target_os = "windows")]
    pub fn set_sysproxy(&mut self, sysproxy: bool) {
        self.state.sysproxy = sysproxy;
    }
}
