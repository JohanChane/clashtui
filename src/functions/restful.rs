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
    pub const DEFAULT_USER_AGENT: &str = "github.com/JohanChane/clashtui";
}

type Result<T, E = minreq::Error> = core::result::Result<T, E>;

pub mod control {
    use super::*;

    /// Restart clash core via http
    ///
    /// usually, an empty str is returned
    pub fn restart(payload: Option<String>) -> Result<()> {
        request(
            Method::Post,
            "/restart",
            Some(payload.unwrap_or(DEFAULT_PAYLOAD.to_string())),
        )
        .map(|_| ())
    }

    /// Get clash core version
    ///
    /// for mihomo, it's like `{"meta": true, "version": "v1.1.1"}`
    pub fn version() -> Result<String> {
        request(Method::Get, "/version", None).and_then(|r| r.as_str().map(|s| s.to_owned()))
    }

    /// Try GET `https://www.gstatic.com/generate_204`
    ///
    /// return nothing on success
    pub fn check_connectivity() -> Result<()> {
        minreq::get("https://www.gstatic.com/generate_204")
            .with_proxy(minreq::Proxy::new(&CONFIG.proxy_addr)?)
            .with_timeout(timeout!())
            .send_lazy()
            .map(|_| ())
    }
}

pub mod config {
    use super::*;

    pub fn fetch() -> Result<config_struct::ClashConfig> {
        request(Method::Get, "/configs", None).and_then(|r| r.json())
    }

    pub fn reload<S: AsRef<str>>(path: S) -> Result<String> {
        request(
            Method::Put,
            "/configs?force=true",
            Some(
                serde_json::json!({
                    "path": path.as_ref(),
                    "payload": ""
                })
                .to_string(),
            ),
        )
        .and_then(|r| r.as_str().map(|s| s.to_owned()))
    }

    pub fn patch(payload: String) -> Result<String> {
        request(Method::Patch, "/configs", Some(payload))
            .and_then(|r| r.as_str().map(|s| s.to_owned()))
    }
}

pub mod download {
    use super::*;

    pub fn profile(url: &str, with_proxy: bool) -> Result<minreq::ResponseLazy> {
        #[cfg(feature = "deprecated")]
        let mut req = parse_request_with_cred(url)?;
        #[cfg(not(feature = "deprecated"))]
        let mut req = minreq::get(url);
        if with_proxy {
            req = req.with_proxy(minreq::Proxy::new(&CONFIG.proxy_addr)?)
        }
        req.with_timeout(timeout!())
            .with_header(
                headers::USER_AGENT,
                CONFIG.global_ua.as_deref().unwrap_or("clash.meta"),
            )
            .send_lazy()
    }

    pub fn github(url: &str, with_proxy: bool, token: String) -> Result<minreq::ResponseLazy> {
        let mut req = minreq::get(url);
        if with_proxy {
            req = req.with_proxy(minreq::Proxy::new(&CONFIG.proxy_addr)?)
        }
        req.with_timeout(timeout!())
            .with_header(headers::AUTHORIZATION, format!("Bearer {token}"))
            .send_lazy()
    }

    pub fn gitlab(url: &str, with_proxy: bool, token: String) -> Result<minreq::ResponseLazy> {
        let mut req = minreq::get(url);
        if with_proxy {
            req = req.with_proxy(minreq::Proxy::new(&CONFIG.proxy_addr)?)
        }
        req.with_timeout(timeout!())
            .with_header("PRIVATE-TOKEN", token)
            .send_lazy()
    }
}

pub mod proxies;

pub mod connection {
    use super::*;

    use serde::Deserialize;

    #[cfg_attr(test, derive(Debug))]
    #[derive(Deserialize, Default)]
    #[serde(rename_all = "camelCase")]
    pub struct ConnInfo {
        pub download_total: u64,
        pub upload_total: u64,
        pub connections: Option<Vec<Conn>>,
    }

    #[cfg_attr(test, derive(Debug))]
    #[derive(Deserialize)]
    pub struct Conn {
        pub id: String,
        pub metadata: ConnMetaData,
        pub upload: u64,
        pub download: u64,
        pub start: String,
        pub chains: Vec<String>,
        #[serde(default)]
        pub rule: Option<String>,
        #[serde(default, rename = "rulePayload")]
        pub rule_payload: Option<String>,
    }

    #[cfg_attr(test, derive(Debug))]
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ConnMetaData {
        pub network: String,
        #[serde(rename = "type")]
        pub ctype: String,
        pub host: String,
        pub process: String,
        pub process_path: String,

        #[serde(rename = "sourceIP")]
        pub source_ip: String,
        pub source_port: String,
        pub remote_destination: String,
        #[serde(default, rename = "destinationPort")]
        pub destination_port: String,
        #[serde(default, rename = "destinationIP")]
        pub destination_ip: Option<String>,
        #[serde(default, rename = "sniffHost")]
        pub sniff_host: Option<String>,
    }

    /// return [ConnInfo]
    pub fn get_connections() -> Result<ConnInfo> {
        request(Method::Get, "/connections", None).and_then(|r| r.json())
    }

    /// Terminate all active connections
    pub fn terminate_all_connections() -> Result<()> {
        request(Method::Delete, "/connections", None).map(|_| ())
    }

    /// if `id` is some, will try to terminate that connection,
    /// otherwise try to terminate **all** connections.
    ///
    /// Return true on success
    ///
    /// NOTE:
    /// Empty str is returned if connection is terminated successfully
    pub fn terminate_connection(id: Option<String>) -> Result<bool> {
        request(
            Method::Delete,
            &format!(
                "/connections{}",
                id.map(|c| format!("/{c}")).unwrap_or_default()
            ),
            None,
        )
        .and_then(|r| {
            r.as_str().map(|s| {
                // try to catch failure
                log::debug!("terminate conn:{s}");
                s.is_empty()
            })
        })
    }
}
