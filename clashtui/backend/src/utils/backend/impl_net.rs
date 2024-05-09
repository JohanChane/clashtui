use super::ClashBackend;
use api::{ClashConfig, Resp};

type Result<T> = core::result::Result<T, String>;

impl ClashBackend {
    pub fn clash_version(&self) -> String {
        match self.clash_api.version() {
            Ok(v) => v,
            Err(e) => {
                log::warn!("{e:?}");
                "Unknown".to_string()
            }
        }
    }
    pub(super) fn fetch_remote(&self) -> Result<ClashConfig> {
        use core::str::FromStr as _;
        self.clash_api.config_get().and_then(|cur_remote| {
            ClashConfig::from_str(cur_remote.as_str())
                .map_err(|e| format!("Failed to prase str:{e:?}"))
        })
    }
    pub fn restart_clash(&self) -> Result<String> {
        self.clash_api.restart(None)
    }
    pub fn dl_remote_profile(&self, url: &str) -> Result<Resp> {
        self.clash_api
            .mock_clash_core(url, self.clash_api.version().is_ok())
    }
    pub fn config_reload(&self, body: String) -> Result<()> {
        self.clash_api.config_reload(body)
    }
}
