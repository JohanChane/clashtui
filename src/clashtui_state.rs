//use crate::utils::_State;
//use std::rc::Rc;
//
//use crate::utils::SharedClashTuiUtil;
//
//
////pub type SharedClashTuiState = Rc<std::cell::RefCell<_ClashTuiState>>;
//
//pub struct _ClashTuiState {
//    state: _State,
//    clashtui_util: SharedClashTuiUtil,
//}
//
//impl _ClashTuiState {
//    pub fn new(clashtui_util: SharedClashTuiUtil) -> Self {
//        let instance = Self {
//            state: clashtui_util.update_state(),
//            clashtui_util,
//        };
//
//        #[cfg(target_os = "windows")]
//        {
//            let sysproxy = ClashTuiUtil::is_system_proxy_enabled().unwrap_or(false);
//            if instance.state.sysproxy {
//                if !sysproxy {
//                    instance.clashtui_util.enable_system_proxy();
//                }
//            } else {
//                if sysproxy {
//                    ClashTuiUtil::disable_system_proxy();
//                }
//            }
//        }
//
//        instance
//    }
//
//    pub fn get_profile(&self) -> &String {
//        &self.state.profile
//    }
//    pub fn set_profile(&mut self, profile: String) {
//        // With update state
//        self.clashtui_util.update_profile_status(profile);
//        self.update_tun();
//    }
//    pub fn update_tun(&mut self){
//        self.state = self.clashtui_util.update_state();
//    }
//    pub fn get_tun(&self) -> String {
//        self.state.tun.clone()
//    }
//    #[cfg(target_os = "windows")]
//    pub fn get_sysproxy(&self) -> bool {
//        self.state.sysproxy
//    }
//    #[cfg(target_os = "windows")]
//    pub fn set_sysproxy(&mut self, sysproxy: bool) {
//        self.state.sysproxy = sysproxy;
//    }
//}
//
