use crate::config::CONFIG;
use minreq::Method;

pub mod config_struct;
pub mod core_detect;
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

pub mod cache {
    use super::*;

    /// Flush fake-IP cache
    ///
    /// API: POST /cache/fakeip/flush
    pub fn flush_fakeip() -> Result<()> {
        request(Method::Post, "/cache/fakeip/flush", None).map(|_| ())
    }

    /// Flush DNS cache
    ///
    /// API: POST /cache/dns/flush
    pub fn flush_dns() -> Result<()> {
        request(Method::Post, "/cache/dns/flush", None).map(|_| ())
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
        .and_then(|r| {
            if r.status_code >= 200 && r.status_code < 300 {
                r.as_str().map(|s| s.to_owned()).map_err(|e| e.into())
            } else {
                let body = r.as_str().unwrap_or("(non-utf8 body)");
                Err(minreq::Error::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("HTTP {}: {body}", r.status_code),
                )))
            }
        })
    }

    pub fn patch(payload: String) -> Result<String> {
        request(Method::Patch, "/configs", Some(payload))
            .and_then(|r| r.as_str().map(|s| s.to_owned()))
    }
}

pub mod download {
    use super::*;

    const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    fn base64_encode(input: &[u8]) -> String {
        let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
        for chunk in input.chunks(3) {
            let b = [chunk[0], chunk.get(1).copied().unwrap_or(0), chunk.get(2).copied().unwrap_or(0)];
            let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | (b[2] as u32);
            out.push(B64[((n >> 18) & 0x3F) as usize] as char);
            out.push(B64[((n >> 12) & 0x3F) as usize] as char);
            out.push(if chunk.len() > 1 { B64[((n >> 6) & 0x3F) as usize] as char } else { '=' });
            out.push(if chunk.len() > 2 { B64[(n & 0x3F) as usize] as char } else { '=' });
        }
        out
    }

    fn strip_userinfo(url: &str) -> (String, Option<String>) {
        let Some(scheme_end) = url.find("://") else {
            return (url.to_string(), None);
        };
        let rest = &url[(scheme_end + 3)..];
        let at_pos = rest.find('@');
        let slash_pos = rest.find('/');
        let is_in_authority = match (at_pos, slash_pos) {
            (Some(a), Some(s)) => a < s,
            (Some(_), None) => true,
            _ => false,
        };
        if !is_in_authority {
            return (url.to_string(), None);
        }
        let userinfo = &rest[..at_pos.unwrap()];
        let auth_value = if userinfo.contains(':') {
            userinfo.to_string()
        } else {
            format!("{userinfo}:")
        };
        let auth_header = format!("Basic {}", base64_encode(auth_value.as_bytes()));
        let prefix = &url[..(scheme_end + 3)];
        let suffix = &rest[(at_pos.unwrap() + 1)..];
        (format!("{prefix}{suffix}"), Some(auth_header))
    }

    pub fn profile(url: &str, with_proxy: bool) -> Result<minreq::ResponseLazy> {
        let (clean_url, auth_header) = strip_userinfo(url);
        let mut req = minreq::get(&clean_url);
        if with_proxy {
            req = req.with_proxy(minreq::Proxy::new(&CONFIG.proxy_addr)?)
        }
        req = req.with_timeout(timeout!())
            .with_header(
                headers::USER_AGENT,
                CONFIG.global_ua.as_deref().unwrap_or_else(|| {
                    if CONFIG.core_type() == crate::config::CoreType::Singbox {
                        "sing-box"
                    } else {
                        "clash.meta"
                    }
                }),
            );
        if let Some(auth) = auth_header {
            req = req.with_header(headers::AUTHORIZATION, auth);
        }
        req.send_lazy()
    }

    pub fn fetch_subscription_userinfo(url: &str, with_proxy: bool) -> Result<Option<String>> {
        let (clean_url, auth_header) = strip_userinfo(url);
        let mut req = minreq::get(&clean_url)
            .with_timeout(timeout!())
            .with_header(
                headers::USER_AGENT,
                CONFIG.global_ua.as_deref().unwrap_or_else(|| {
                    if CONFIG.core_type() == crate::config::CoreType::Singbox {
                        "sing-box"
                    } else {
                        "clash.meta"
                    }
                }),
            );
        if with_proxy {
            req = req.with_proxy(minreq::Proxy::new(&CONFIG.proxy_addr)?);
        }
        if let Some(auth) = auth_header {
            req = req.with_header(headers::AUTHORIZATION, auth);
        }
        let resp = req.send()?;
        let info: Option<String> = resp
            .headers
            .get("subscription-userinfo")
            .cloned();
        Ok(info)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn strip_token_from_github_url() {
            let url = "https://ghp_token@raw.githubusercontent.com/user/repo/main/config.yaml";
            let (clean, auth) = strip_userinfo(url);
            assert_eq!(clean, "https://raw.githubusercontent.com/user/repo/main/config.yaml");
            assert_eq!(auth.unwrap(), "Basic Z2hwX3Rva2VuOg==");
        }

        #[test]
        fn strip_user_pass_from_url() {
            let url = "https://user:pass@example.com/path";
            let (clean, auth) = strip_userinfo(url);
            assert_eq!(clean, "https://example.com/path");
            assert_eq!(auth.unwrap(), "Basic dXNlcjpwYXNz");
        }

        #[test]
        fn no_userinfo_no_change() {
            let url = "https://example.com/path";
            let (clean, auth) = strip_userinfo(url);
            assert_eq!(clean, url);
            assert!(auth.is_none());
        }

        #[test]
        fn at_in_path_not_userinfo() {
            let url = "https://example.com/path?q=@test";
            let (clean, auth) = strip_userinfo(url);
            assert_eq!(clean, url);
            assert!(auth.is_none());
        }
    }
}

pub mod proxies;

pub mod connection {
    use super::*;

    use serde::Deserialize;

    #[cfg_attr(test, derive(Debug))]
    #[derive(Deserialize, Default)]
    #[serde(rename_all = "camelCase", default)]
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
        #[serde(rename = "type", default)]
        pub ctype: String,
        pub host: String,
        #[serde(default)]
        pub process: String,
        #[serde(default)]
        pub process_path: String,

        #[serde(rename = "sourceIP")]
        pub source_ip: String,
        pub source_port: String,
        #[serde(default)]
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

pub mod api_log {
    use super::*;

    pub struct LogEntry {
        pub type_: String,
        pub payload: String,
        pub time: String,
    }

    pub(crate) fn timestamp() -> String {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let days = secs / 86400;
        let time_of_day = secs % 86400;
        let hh = time_of_day / 3600;
        let mm = (time_of_day % 3600) / 60;
        let ss = time_of_day % 60;

        let mut y: i64 = 1970;
        let mut remaining_days = days;
        loop {
            let days_in_year = if is_leap(y) { 366 } else { 365 };
            if remaining_days < days_in_year {
                break;
            }
            remaining_days -= days_in_year;
            y += 1;
        }
        let dims: &[i64] = if is_leap(y) {
            &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        } else {
            &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        };
        let mut mo = 1;
        for dim in dims {
            if remaining_days < *dim {
                break;
            }
            remaining_days -= dim;
            mo += 1;
        }
        let yy = y % 100;
        let dd = remaining_days + 1;
        format!("{yy:02}-{mo:02}-{dd:02} {hh:02}:{mm:02}:{ss:02}")
    }

    fn is_leap(y: i64) -> bool {
        (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
    }

    pub fn parse_log_entries(body: &str) -> Vec<LogEntry> {
        body.lines()
            .filter(|line| !line.is_empty())
            .filter_map(|line| match serde_json::from_str::<serde_json::Value>(line) {
                Ok(v) => {
                    let type_ = v
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("unknown")
                        .to_owned();
                    let payload = v
                        .get("payload")
                        .and_then(|p| p.as_str())
                        .unwrap_or("")
                        .to_owned();
                    Some(LogEntry {
                        type_,
                        payload,
                        time: timestamp(),
                    })
                }
                Err(_) => {
                    log::warn!("Failed to parse log line as JSON: {line}");
                    None
                }
            })
            .collect()
    }

    pub fn get_logs(level: Option<&str>) -> Result<Vec<LogEntry>> {
        let url = match level {
            Some(l) if !l.is_empty() && l != "unknown" => format!("/logs?level={l}"),
            _ => "/logs".to_owned(),
        };
        request(Method::Get, &url, None).and_then(|r| {
            let body = r.as_str()?;
            Ok(parse_log_entries(body))
        })
    }
}
