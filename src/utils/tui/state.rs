use super::SharedClashTuiUtil;

#[cfg(target_os = "linux")]
pub struct _State {
    pub profile: String,
    pub mode: String,
    pub tun: String,
    pub ver: String,
}
#[cfg(target_os = "linux")]
impl _State {
    pub fn new(pf: String, mode: String, tun: String, ver: String) -> Self {
        Self {
            tun,
            mode,
            profile: pf,
            ver,
        }
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
        // With update state
        self.st = self.ct.update_state(Some(profile), None);
    }
    pub fn set_mode(&mut self, mode: String) {
        self.st = self.ct.update_state(None, Some(mode));
    }
    pub fn render(&self) -> String {
        #[cfg(target_os = "windows")]
        let status_str = format!(
            "Profile: {}    Tun: {}    SysProxy: {}    Help: ?",
            self.get_profile(),
            self.get_tun(),
            self.get_sysproxy().to_string(),
        );
        #[cfg(target_os = "linux")]
        let status_str = format!(
            "Profile: {}    Mode: {}    Tun: {}    ClashVer: {}    Help: ?",
            self.st.profile, self.st.mode, self.st.tun, self.st.ver
        );
        status_str
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
