use super::*;
use indexmap::IndexMap;
use serde::Deserialize;
use std::collections::HashMap;

const DEFAULT_TEST_URL: &str = "https://www.gstatic.com/generate_204";

fn encode_path(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{:02X}", b),
        })
        .collect::<String>()
}

fn encode_query(s: &str) -> String {
    encode_path(s)
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ProxyCatalog {
    #[serde(rename = "proxies")]
    pub primary: IndexMap<String, Proxy>,
    #[serde(skip)]
    pub providers: IndexMap<String, ProxyProvider>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ProxyProvider {
    #[serde(rename = "vehicleType", default)]
    pub vehicle_type: String,
    #[serde(rename = "testUrl", default)]
    pub test_url: Option<String>,
    #[serde(default)]
    pub proxies: Vec<Proxy>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyTarget {
    pub name: String,
    pub provider: Option<String>,
    // An ID changes when mihomo replaces a node during a provider update.
    pub id: Option<String>,
}

pub struct ResolvedProxy<'a> {
    pub proxy: &'a Proxy,
    pub provider: Option<&'a str>,
    pub test_url: Option<&'a str>,
}

impl ResolvedProxy<'_> {
    pub fn target(&self) -> ProxyTarget {
        ProxyTarget {
            name: self.proxy.name.clone(),
            provider: self.provider.map(str::to_owned),
            id: self.proxy.id.clone(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum LookupError {
    Missing,
    Ambiguous,
}

impl LookupError {
    pub fn message(&self, name: &str) -> String {
        match self {
            Self::Missing => format!("Proxy {name} is unavailable; refresh and try again"),
            Self::Ambiguous => {
                format!("Multiple proxies named {name}; test the parent group instead")
            }
        }
    }
}

impl ProxyCatalog {
    /// Group `all` and `now` contain names only, without provider identities.
    /// Resolve only an unambiguous object; never choose a provider by map order.
    pub fn resolve(&self, name: &str) -> Result<ResolvedProxy<'_>, LookupError> {
        let primary = self.primary.get(name);
        let mut resolved = primary.map(|proxy| ResolvedProxy {
            proxy,
            provider: None,
            test_url: proxy.test_url.as_deref().filter(|url| !url.is_empty()),
        });
        for (provider_name, provider) in &self.providers {
            // Compatible providers are aliases of primary/real provider objects.
            if provider.vehicle_type == "Compatible" {
                continue;
            }
            for proxy in provider.proxies.iter().filter(|p| p.name == name) {
                if let Some(previous) = &resolved {
                    let same_id = match (&previous.proxy.id, &proxy.id) {
                        (Some(a), Some(b)) if !a.is_empty() && !b.is_empty() => a == b,
                        _ => {
                            previous.provider.is_none()
                                && previous.proxy.provider_name.as_deref()
                                    == Some(provider_name.as_str())
                        }
                    };
                    if !same_id {
                        return Err(LookupError::Ambiguous);
                    }
                }
                resolved = Some(ResolvedProxy {
                    proxy,
                    provider: Some(provider_name),
                    test_url: proxy
                        .test_url
                        .as_deref()
                        .filter(|url| !url.is_empty())
                        .or(provider.test_url.as_deref().filter(|url| !url.is_empty())),
                });
            }
        }
        resolved.ok_or(LookupError::Missing)
    }

    pub fn record_delay(&mut self, target: &ProxyTarget, delay: u64) {
        let proxy = if let Some(provider) = &target.provider {
            self.providers
                .get_mut(provider)
                .and_then(|provider| provider.proxies.iter_mut().find(|p| p.name == target.name))
        } else {
            self.primary.get_mut(&target.name)
        };
        if let Some(proxy) = proxy
            && proxy.id == target.id
        {
            proxy.history.push(DelayRecord { delay });
        }
    }

    pub fn record_group_delays(
        &mut self,
        before_test: &Self,
        group: &str,
        delays: &HashMap<String, u64>,
    ) {
        let Some(members) = before_test.primary.get(group).and_then(|p| p.all.as_ref()) else {
            return;
        };
        for (name, delay) in delays {
            if members.contains(name)
                && let Ok(proxy) = before_test.resolve(name)
            {
                self.record_delay(&proxy.target(), *delay);
            }
        }
    }
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Proxy {
    #[allow(dead_code)]
    pub name: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub proxy_type: String,
    #[serde(default)]
    #[cfg_attr(not(test), allow(dead_code))]
    pub alive: bool,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub now: Option<String>,
    #[serde(default)]
    pub all: Option<Vec<String>>,
    #[serde(default)]
    pub history: Vec<DelayRecord>,
    #[allow(dead_code)]
    #[serde(default)]
    pub extra: IndexMap<String, DelayInfo>,
    #[serde(default, alias = "testUrl")]
    pub test_url: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    pub provider_name: Option<String>,
    #[serde(default)]
    pub udp: bool,
    #[serde(default)]
    pub tcp: bool,
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
        let delay = v
            .get("delay")
            .and_then(|d| {
                d.as_u64()
                    .or_else(|| d.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(0);
        Ok(DelayRecord { delay })
    }
}

#[derive(Debug, Clone)]
pub struct DelayInfo {
    #[allow(dead_code)]
    pub alive: bool,
    #[allow(dead_code)]
    pub history: Vec<DelayRecord>,
}

impl<'de> Deserialize<'de> for DelayInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = serde_json::Value::deserialize(deserializer)?;
        let alive = v.get("alive").and_then(|a| a.as_bool()).unwrap_or(false);
        let history = v
            .get("history")
            .and_then(|h| serde_json::from_value(h.clone()).ok())
            .unwrap_or_default();
        Ok(DelayInfo { alive, history })
    }
}

fn get_json(path: &str) -> Result<serde_json::Value> {
    let response = request(Method::Get, path, None)?;
    if !(200..300).contains(&response.status_code) {
        return Err(minreq::Error::IoError(std::io::Error::other(format!(
            "HTTP {} from {}",
            response.status_code,
            path.split('?').next().unwrap_or(path)
        ))));
    }
    response.json()
}

pub fn fetch_proxies() -> Result<ProxyCatalog> {
    fetch_proxies_with(get_json)
}

fn fetch_proxies_with(
    mut fetch: impl FnMut(&str) -> Result<serde_json::Value>,
) -> Result<ProxyCatalog> {
    let mut catalog: ProxyCatalog =
        serde_json::from_value(fetch("/proxies")?).map_err(minreq::Error::SerdeJsonError)?;
    // sing-box and older Clash API implementations may not expose providers.
    if let Ok(response) = fetch("/providers/proxies")
        && let Some(providers) = response.get("providers").and_then(|v| v.as_object())
    {
        for (name, value) in providers {
            if let Ok(provider) = serde_json::from_value(value.clone()) {
                catalog.providers.insert(name.clone(), provider);
            }
        }
    }
    Ok(catalog)
}

pub fn select_proxy(group: &str, node: &str) -> Result<()> {
    let payload = serde_json::json!({ "name": node }).to_string();
    request(
        Method::Put,
        &format!("/proxies/{}", encode_path(group)),
        Some(payload),
    )
    .map(|_| ())
}

fn proxy_delay_endpoint(target: &ProxyTarget, url: Option<&str>, timeout: u64) -> String {
    let name_enc = encode_path(&target.name);
    let test_url = url
        .filter(|url| !url.is_empty())
        .unwrap_or(DEFAULT_TEST_URL);
    let path = match &target.provider {
        Some(provider) => format!(
            "/providers/proxies/{}/{name_enc}/healthcheck",
            encode_path(provider)
        ),
        None => format!("/proxies/{name_enc}/delay"),
    };
    format!("{path}?url={}&timeout={timeout}", encode_query(test_url))
}

pub fn test_proxy_delay(
    target: &ProxyTarget,
    url: Option<&str>,
    timeout: u64,
) -> Result<Option<u64>> {
    let value = get_json(&proxy_delay_endpoint(target, url, timeout))?;
    Ok(value.get("delay").and_then(|d| {
        d.as_u64()
            .or_else(|| d.as_str().and_then(|s| s.parse().ok()))
    }))
}

pub fn test_group_delay(
    name: &str,
    url: Option<&str>,
    timeout: u64,
) -> Result<HashMap<String, u64>> {
    let name_enc = encode_path(name);
    let test_url = url
        .filter(|url| !url.is_empty())
        .unwrap_or(DEFAULT_TEST_URL);
    let endpoint = format!(
        "/group/{name_enc}/delay?url={}&timeout={timeout}",
        encode_query(test_url)
    );
    get_json(&endpoint).map(|v| {
        v.as_object()
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| {
                        let delay = v
                            .as_u64()
                            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))?;
                        Some((k.clone(), delay))
                    })
                    .collect()
            })
            .unwrap_or_default()
    })
}

#[cfg(test)]
mod tests;
