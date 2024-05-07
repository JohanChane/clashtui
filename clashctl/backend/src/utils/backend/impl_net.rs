use super::ClashBackend;
use api::{ClashConfig, Resp};
use std::io::Error;

impl ClashBackend {
    pub fn clash_version(&self) -> String {
        match self.clash_api.version() {
            Ok(v) => v,
            Err(e) => {
                log::warn!("{}", e);
                "Unknown".to_string()
            }
        }
    }
    pub(super) fn fetch_remote(&self) -> Result<ClashConfig, Error> {
        use core::str::FromStr as _;
        self.clash_api.config_get().and_then(|cur_remote| {
            ClashConfig::from_str(cur_remote.as_str())
                .map_err(|_| Error::new(std::io::ErrorKind::InvalidData, "Failed to prase str"))
        })
    }
    pub fn restart_clash(&self) -> Result<String, Error> {
        self.clash_api.restart(None)
    }
    pub fn dl_remote_profile(&self, url: &str) -> Result<Resp, Error> {
        self.clash_api
            .mock_clash_core(url, self.clash_api.version().is_ok())
    }
    pub fn config_reload(&self, body: String) -> Result<(), Error> {
        self.clash_api.config_reload(body)
    }
}
