use serde::{Deserialize, Serialize};
use std::fs::File;
use std::rc::Rc;

use crate::utils::SharedClashTuiUtil;

#[derive(Serialize, Deserialize, Default)]
struct State {
    pub profile: String,
    pub tun: String,
    pub sysproxy: bool,
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
        instance.state.tun = "Unknown".to_string(); // tun default init value is Unknown

        instance.load_status_from_file();

        instance.set_tun(instance.clashtui_util.get_tun_mode());

        #[cfg(target_os = "windows")]
        {
            let sysproxy = ClashTuiUtil::is_system_proxy_enabled().unwrap_or(false);
            if instance.state.sysproxy {
                if !sysproxy {
                    instance.clashtui_util.enable_system_proxy();
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
        let path = self.clashtui_util.clashtui_dir.join("clashtui_status.yaml");

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
        let path = self.clashtui_util.clashtui_dir.join("clashtui_status.yaml");

        if let Ok(status_file) = File::create(&path) {
            serde_yaml::to_writer(status_file, &self.state).unwrap();
        }
    }

    pub fn get_profile(&self) -> &String {
        &self.state.profile
    }
    pub fn set_profile(&mut self, profile: String) {
        self.state.profile = profile;
        self.update_tun();
    }
    fn update_tun(&mut self){
        self.state.tun = self.clashtui_util.get_tun_mode();
    }
    pub fn get_tun(&self) -> String {
        self.state.tun.clone()
    }
    pub fn set_tun(&mut self, tun: String) {
        self.state.tun = tun;
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
