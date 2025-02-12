use crate::clash::webapi::{Mode, TunStack};

pub struct State {
    pub profile: String,
    pub mode: Option<Mode>,
    pub tun: Option<TunStack>,
    #[cfg(target_os = "windows")]
    pub sysproxy: Option<bool>,
}

impl State {
    pub fn unknown(profile: String) -> Self {
        Self {
            profile,
            mode: None,
            tun: None,
            #[cfg(target_os = "windows")]
            sysproxy: None,
        }
    }
}

impl core::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mode_str = self
            .mode
            .as_ref()
            .map_or("Unknown".to_string(), |v| v.to_string());
        let tun_str = self
            .tun
            .as_ref()
            .map_or("Unknown".to_string(), |v| v.to_string());

        #[cfg(target_os = "windows")]
        let sysproxy_str = self
            .sysproxy
            .map_or("Unknown".to_string(), |v| v.to_string());

        #[cfg(target_os = "windows")]
        write!(
            f,
            "Profile: {}    Mode: {}    SysProxy: {}    Tun: {}    Help: ?",
            self.profile, mode_str, sysproxy_str, tun_str
        )?;

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        write!(
            f,
            "Profile: {}    Mode: {}    Tun: {}    Help: ?",
            self.profile, mode_str, tun_str
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unknown_state_display() {
        let state = State::unknown("Default".to_string());
        assert_eq!(
            format!("{}", state),
            "Profile: Default    Mode: Unknown    Tun: Unknown    Help: ?"
        );
    }

    #[test]
    fn test_state_display_with_mode_and_tun() {
        let mut state = State::unknown("Default".to_string());
        state.mode = Some(Mode::Global);
        state.tun = Some(TunStack::Mixed);
        assert_eq!(
            format!("{}", state),
            "Profile: Default    Mode: Global    Tun: Mixed    Help: ?"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_state_display_with_sysproxy() {
        let mut state = State::unknown("Default".to_string());
        state.sysproxy = Some(true);
        assert_eq!(
            format!("{}", state),
            "Profile: Default    Mode: Unknown    SysProxy: true    Tun: Unknown    Help: ?"
        );
    }
}
