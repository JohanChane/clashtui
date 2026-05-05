use super::*;
use indexmap::IndexMap;
use serde::Deserialize;
use std::collections::HashMap;

const DEFAULT_TEST_URL: &str = "https://www.gstatic.com/generate_204";

fn encode_path(s: &str) -> String {
    s.bytes().map(|b| match b {
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
        _ => format!("%{:02X}", b),
    }).collect::<String>()
}

fn encode_query(s: &str) -> String {
    s.bytes().map(|b| match b {
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b':' | b'/' | b'%' => (b as char).to_string(),
        _ => format!("%{:02X}", b),
    }).collect::<String>()
}

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

#[derive(Debug, Clone)]
pub struct DelayRecord {
    pub delay: u64,
}

impl<'de> Deserialize<'de> for DelayRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = serde_json::Value::deserialize(deserializer)?;
        let delay = v.get("delay").and_then(|d| {
            d.as_u64().or_else(|| d.as_str().and_then(|s| s.parse().ok()))
        }).unwrap_or(0);
        Ok(DelayRecord { delay })
    }
}

#[derive(Debug, Clone)]
pub struct DelayInfo {
    pub alive: bool,
    pub history: Vec<DelayRecord>,
}

impl<'de> Deserialize<'de> for DelayInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = serde_json::Value::deserialize(deserializer)?;
        let alive = v.get("alive").and_then(|a| a.as_bool()).unwrap_or(false);
        let history = v.get("history")
            .and_then(|h| serde_json::from_value(h.clone()).ok())
            .unwrap_or_default();
        Ok(DelayInfo { alive, history })
    }
}

pub fn fetch_proxies() -> Result<ProxiesResponse> {
    request(Method::Get, "/proxies", None).and_then(|r| r.json())
}

pub fn get_proxy(name: &str) -> Result<Proxy> {
    request(Method::Get, &format!("/proxies/{}", encode_path(name)), None).and_then(|r| r.json())
}

pub fn select_proxy(group: &str, node: &str) -> Result<()> {
    let payload = serde_json::json!({ "name": node }).to_string();
    request(Method::Put, &format!("/proxies/{}", encode_path(group)), Some(payload)).map(|_| ())
}

pub fn test_proxy_delay(name: &str, url: Option<&str>, timeout: u64) -> Result<Option<u64>> {
    let name_enc = encode_path(name);
    let test_url = url.unwrap_or(DEFAULT_TEST_URL);
    let endpoint = format!(
        "/proxies/{name_enc}/delay?url={}&timeout={timeout}",
        encode_query(test_url)
    );
    request(Method::Get, &endpoint, None).and_then(|r| {
        let v: serde_json::Value = r.json()?;
        let delay = v.get("delay").and_then(|d| {
            d.as_u64().or_else(|| d.as_str().and_then(|s| s.parse().ok()))
        });
        Ok(delay.filter(|&d| d > 0))
    })
}

pub fn test_group_delay(name: &str, url: Option<&str>, timeout: u64) -> Result<HashMap<String, u64>> {
    let name_enc = encode_path(name);
    let test_url = url.unwrap_or(DEFAULT_TEST_URL);
    let endpoint = format!(
        "/group/{name_enc}/delay?url={}&timeout={timeout}",
        encode_query(test_url)
    );
    request(Method::Get, &endpoint, None).and_then(|r| {
        let v: serde_json::Value = r.json()?;
        let map = v.as_object().map(|obj| {
            obj.iter().filter_map(|(k, v)| {
                let delay = v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))?;
                if delay > 0 { Some((k.clone(), delay)) } else { None }
            }).collect()
        }).unwrap_or_default();
        Ok(map)
    })
}
