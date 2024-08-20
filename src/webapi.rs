mod impls;
mod structs;

pub use structs::{ClashConfig, LogLevel, Mode, TunConfig, TunStack};

const TIMEOUT: u64 = 5;

#[derive(Debug)]
pub struct ClashUtil {
    api: String,
    secret: Option<String>,
    ua: Option<String>,
    timeout: u64,
    pub proxy_addr: String,
}

impl ClashUtil {
    pub fn new(
        controller_api: String,
        secret: Option<String>,
        proxy_addr: String,
        ua: Option<String>,
        timeout: Option<u64>,
    ) -> Self {
        Self {
            api: controller_api,
            secret,
            ua,
            proxy_addr,
            timeout: timeout.unwrap_or(TIMEOUT),
        }
    }
}
