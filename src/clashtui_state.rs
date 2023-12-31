use serde::{Deserialize, Serialize};
use std::fs::File;
use std::rc::Rc;

use crate::utils::SharedClashTuiUtil;

// save state (include user's choices) of clashtui.
#[derive(Serialize, Deserialize, Default)]
struct State {
    pub profile: String,
    pub tun: bool,      // user's choice. indicates whether to enable the 'tun' mode.
    pub sysproxy: bool, // user's choice. indicates whether to enable system proxy.
}

pub type SharedClashTuiState = Rc<std::cell::RefCell<ClashTuiState>>;

pub struct ClashTuiState {
    state: State,
    clashtui_util: SharedClashTuiUtil,
}

impl ClashTuiState {
    pub fn new(clashtui_util: SharedClashTuiUtil) -> Self {
        let mut instance = Self {
            state: State::default(),
            clashtui_util,
        };
        instance.state.tun = true; // tun default init value is true

        instance.load_status_from_file();
        // correct tun info of clashtui state using mihomo api when starting
        let tun_info = instance.clashtui_util.borrow().get_tun_mode();
        if let Some(tun_info) = tun_info {
            instance.set_tun(tun_info.0);
        }

        #[cfg(target_os = "windows")]
        {
            let sysproxy = ClashTuiUtil::is_system_proxy_enabled().unwrap_or(false);
            if instance.state.sysproxy {
                if !sysproxy {
                    instance.clashtui_util.borrow().enable_system_proxy();
                }
            } else {
                if sysproxy {
                    ClashTuiUtil::disable_system_proxy();
                }
            }
        }

        instance
    }

    pub fn load_status_from_file(&mut self) {
        let path = self
            .clashtui_util
            .borrow()
            .clashtui_dir
            .join("clashtui_status.yaml");

        if let Ok(file) = File::open(&path) {
            match serde_yaml::from_reader::<_, State>(file) {
                Ok(status_file) => {
                    self.state = status_file;
                }
                Err(err) => {
                    log::error!("Error loading YAML: {:?}", err);
                }
            }
        }
    }

    pub fn save_status_to_file(&self) {
        let path = self
            .clashtui_util
            .borrow()
            .clashtui_dir
            .join("clashtui_status.yaml");

        if let Ok(status_file) = File::create(&path) {
            serde_yaml::to_writer(status_file, &self.state).unwrap();
        }
    }

    pub fn get_profile(&self) -> &String {
        &self.state.profile
    }
    pub fn set_profile(&mut self, profile: String) {
        self.state.profile = profile;
    }
    pub fn get_tun(&self) -> bool {
        self.state.tun
    }
    pub fn set_tun(&mut self, tun: bool) {
        self.state.tun = tun;
    }
    pub fn get_sysproxy(&self) -> bool {
        self.state.sysproxy
    }
    pub fn set_sysproxy(&mut self, sysproxy: bool) {
        self.state.sysproxy = sysproxy;
    }
}
