use super::*;
use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct ProxiesResponse {
    pub proxies: IndexMap<String, Proxy>,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Proxy {
    pub name: String,
    #[serde(rename = "type")]
    pub proxy_type: String,
    pub alive: bool,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub now: Option<String>,
    #[serde(default)]
    pub all: Option<Vec<String>>,
    #[serde(default)]
    pub history: Vec<DelayRecord>,
    #[serde(default)]
    pub extra: IndexMap<String, DelayInfo>,
    #[serde(default)]
    pub test_url: Option<String>,
    #[serde(default)]
    pub provider_name: Option<String>,
    #[serde(default)]
    pub udp: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DelayRecord {
    pub delay: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DelayInfo {
    pub alive: bool,
    #[serde(default)]
    pub history: Vec<DelayRecord>,
}

#[derive(Debug, Deserialize)]
struct DelayResponse {
    delay: u64,
}

pub fn fetch_proxies() -> Result<ProxiesResponse> {
    request(Method::Get, "/proxies", None).and_then(|r| r.json())
}

pub fn get_proxy(name: &str) -> Result<Proxy> {
    request(Method::Get, &format!("/proxies/{name}"), None).and_then(|r| r.json())
}

pub fn select_proxy(group: &str, node: &str) -> Result<()> {
    let payload = serde_json::json!({ "name": node }).to_string();
    request(Method::Put, &format!("/proxies/{group}"), Some(payload)).map(|_| ())
}

pub fn test_proxy_delay(name: &str, url: &str, timeout: u64) -> Result<u64> {
    let endpoint = format!("/proxies/{name}/delay?url={url}&timeout={timeout}");
    request(Method::Get, &endpoint, None)
        .and_then(|r| r.json::<DelayResponse>())
        .map(|dr| dr.delay)
}

pub fn test_group_delay(name: &str, url: &str, timeout: u64) -> Result<()> {
    let endpoint = format!("/group/{name}/delay?url={url}&timeout={timeout}");
    request(Method::Get, &endpoint, None).map(|_| ())
}
