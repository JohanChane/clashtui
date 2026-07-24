use crate::config::CONFIG;
use minreq::Method;

pub mod config_struct;
#[macro_use]
mod utils;

use utils::*;

const DEFAULT_PAYLOAD: &str = r#"'{"path": "", "payload": ""}'"#;
const DEFAULT_TIMEOUT: u64 = 5;

mod headers {
    pub const USER_AGENT: &str = "user-agent";
    pub const AUTHORIZATION: &str = "authorization";
    pub const CONTENT_TYPE: &str = "Content-Type";
}

type Result<T, E = minreq::Error> = core::result::Result<T, E>;

pub mod control {
    use super::*;

    /// Restart clash core via http
    pub fn restart() -> Result<()> {
        request_return_204(Method::Post, "/restart", Some(DEFAULT_PAYLOAD.to_string()))
    }

    /// Get clash core version
    ///
    /// for mihomo, it's like `{"meta": true, "version": "v1.1.1"}`
    pub fn version() -> Result<String> {
        request(Method::Get, "/version", None).and_then(|r| r.as_str().map(|s| s.to_owned()))
    }

    /// Detect which core is actually running by querying `/version`.
    ///
    /// Checks the `version` field: sing-box reports e.g. `"sing-box 1.13.11"`,
    /// mihomo reports e.g. `"v1.18.10"`.
    pub fn detect_core_type() -> Result<crate::config::CoreType> {
        control::version().and_then(|r| {
            if r.contains("sing-box") {
                Ok(crate::config::CoreType::Singbox)
            } else {
                Ok(crate::config::CoreType::Mihomo)
            }
        })
    }
}

pub mod cache {
    use super::*;

    /// Flush fake-IP cache
    ///
    /// API: POST /cache/fakeip/flush
    pub fn flush_fakeip() -> Result<()> {
        request_return_204(Method::Post, "/cache/fakeip/flush", None)
    }

    /// Flush DNS cache
    ///
    /// API: POST /cache/dns/flush
    pub fn flush_dns() -> Result<()> {
        request_return_204(Method::Post, "/cache/dns/flush", None)
    }
}

pub mod config {
    use super::*;

    pub fn fetch() -> Result<config_struct::ClashConfig> {
        request(Method::Get, "/configs", None).and_then(|r| r.json())
    }

    pub fn reload() -> Result<()> {
        request_return_204(Method::Put, "/configs?force=true", None)
    }

    pub fn patch(payload: String) -> Result<()> {
        request_return_204(Method::Patch, "/configs", Some(payload))
    }

    /// Update GEO databases
    ///
    /// API: POST /configs/geo
    pub fn upgrade_geo() -> Result<()> {
        request_return_204(
            Method::Post,
            "/configs/geo",
            Some(DEFAULT_PAYLOAD.to_string()),
        )
    }
}

pub mod download;

pub mod proxies;

pub mod connection;

#[allow(unused)]
pub mod api_log {
    use super::{CONFIG, Result};
    pub use super::{config_struct::LogLevel, utils::WSerr};
    use tokio::sync::Semaphore;

    #[derive(serde::Deserialize, Clone)]
    pub struct LogMessage {
        time: String,
        level: LogLevel,
        message: String,
        fields: String,
    }

    type R = circular_buffer::HeapCircularBuffer<LogMessage>;
    type RM = std::sync::Mutex<R>;
    type RL = std::sync::LazyLock<RM>;

    pub static LOG_POOL: RL = RL::new(|| RM::new(R::with_capacity(256)));
    static PAUSE: Semaphore = Semaphore::const_new(1);

    /// Return a guard that 'pause' the log record (by discarding all incoming logs),
    /// drop the guard to continue recording
    pub fn pause() -> tokio::sync::SemaphorePermit<'static> {
        PAUSE.try_acquire().unwrap()
    }
    /// Return logs from old to new, with its content ranging from
    /// `start` to `start+length` (counting from newest)
    ///
    /// e.g. [3,4,5] for start=3 and length=3
    pub fn get(start: usize, length: usize) -> Vec<LogMessage> {
        LOG_POOL
            .lock()
            .unwrap()
            .iter()
            .skip(start)
            .take(length)
            .rev()
            .cloned()
            .collect()
    }
    /// Call this to start a background thread that collect logs
    ///
    /// Return a `rx` that recv the Error. If error, call this again
    /// to restart
    ///
    /// Note: log level shouldn't be silent
    pub fn subscrbe(level: LogLevel) -> Result<WSerr> {
        debug_assert!(
            matches!(level, LogLevel::Silent),
            "log level shouldn't be silent"
        );
        super::utils::listen_ws(
            format!("/logs?format=structured?level={level}"),
            |msg| {
                LOG_POOL.lock().unwrap().push_front(msg);
            },
            &PAUSE,
        )
    }
}
