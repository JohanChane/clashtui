use super::ClashBackend;
use crate::utils::State;
use api::{ClashConfig, Resp};

type Result<T> = core::result::Result<T, String>;

impl ClashBackend {
    pub fn update_state(
        &self,
        new_pf: Option<String>,
        new_mode: Option<String>,
        #[cfg(target_os = "windows")] new_sysp: Option<bool>,
    ) -> State {
        #[cfg(target_os = "windows")]
        use crate::utils::ipc;
        #[cfg(target_os = "windows")]
        if let Some(b) = new_sysp {
            let _ = if b {
                ipc::enable_system_proxy(&self.clash_api.proxy_addr)
            } else {
                ipc::disable_system_proxy()
            };
        }
        let (profile, mode, tun) = self._update_state(new_pf, new_mode);
        #[cfg(target_os = "windows")]
        let sysp = ipc::is_system_proxy_enabled().map_or_else(
            |v| {
                log::error!("{}", v);
                None
            },
            Some,
        );
        State {
            profile,
            mode,
            tun,
            #[cfg(target_os = "windows")]
            sysproxy: sysp,
        }
    }
    pub fn clash_version(&self) -> String {
        match self.clash_api.version() {
            Ok(v) => v,
            Err(e) => {
                log::warn!("{e:?}");
                "Unknown".to_string()
            }
        }
    }
    pub fn fetch_remote(&self) -> Result<ClashConfig> {
        use core::str::FromStr as _;
        self.clash_api.config_get().and_then(|cur_remote| {
            ClashConfig::from_str(cur_remote.as_str())
                .map_err(|e| format!("Failed to prase str:{e:?}"))
        })
    }
    pub fn restart_clash(&self) -> Result<String> {
        self.clash_api.restart(None)
    }
    pub fn dl_remote_profile(&self, url: &str, with_proxy: bool) -> Result<Resp> {
        self.clash_api.mock_clash_core(
            url,
            with_proxy
                && self.clash_api.version().is_ok()
                && self.clash_api.check_connectivity().is_ok(),
        )
    }
    pub fn config_reload(&self, body: String) -> Result<()> {
        self.clash_api.config_reload(body)
    }
    fn _update_state(
        &self,
        new_pf: Option<String>,
        new_mode: Option<String>,
    ) -> (String, Option<api::Mode>, Option<api::TunStack>) {
        if let Some(v) = new_mode {
            let load = format!(r#"{{"mode": "{}"}}"#, v);
            let _ = self
                .clash_api
                .config_patch(load)
                .map_err(|e| log::error!("Patch Errr:{e:?}"));
        }

        let pf = match new_pf {
            Some(v) => {
                self.cfg.update_profile(&v);
                v
            }
            None => self.cfg.current_profile.borrow().clone(),
        };
        let clash_cfg = self
            .fetch_remote()
            .map_err(|e| log::warn!("Fetch Remote:{e:?}"))
            .ok();
        log::debug!("Fetch Remote:{clash_cfg:?}");
        let (mode, tun) = match clash_cfg {
            Some(v) => (
                Some(v.mode),
                if v.tun.enable {
                    Some(v.tun.stack)
                } else {
                    None
                },
            ),
            None => (None, None),
        };
        (pf, mode, tun)
    }
}
