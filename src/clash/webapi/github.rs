use serde::Deserialize;

use super::ClashUtil;
use crate::CResult;

impl ClashUtil {
    pub fn get_github_info(&self, request: &Request) -> CResult<Response> {
        use super::headers;
        self.get_blob(request.as_url(), None, Some(headers::DEFAULT_USER_AGENT))
            .and_then(|r| serde_json::from_reader(r).map_err(|e| e.into()))
    }
    pub fn get_file(&self, url: &str) -> CResult<minreq::ResponseLazy> {
        use super::headers;
        self.get_blob(url, None, Some(headers::DEFAULT_USER_AGENT))
    }
    /// check self and clash(current:`mihomo`)
    pub fn check_update(&self) -> CResult<Vec<(Response, String)>> {
        let mut clashtui = self.get_github_info(&Request::s_clashtui())?;
        let mut clash = self.get_github_info(&Request::s_clash())?;
        let mut vec = Vec::with_capacity(2);
        if clashtui.is_newer_than(crate::consts::VERSION) {
            clashtui.name = "ClashTUI".to_string();
            vec.push((clashtui.filter_asserts(), crate::consts::VERSION.to_owned()));
        }
        let clash_core_version = match self.version() {
            Ok(v) => {
                let v = serde_json::to_value(v)?;
                // test mihomo
                let mihomo = v.get("version").and_then(|v| v.as_str());
                // try get any
                None.or(mihomo).map(|s| s.to_owned())
            }
            Err(_) => None,
        }
        // if None is get, assume there is no clash core installed/running
        .unwrap_or("v0.0.0".to_owned());
        if clash.is_newer_than(&clash_core_version) {
            clash.name = "Clash Core".to_string();
            vec.push((clash.filter_asserts(), clash_core_version))
        }
        Ok(vec)
    }
}

/// e.g. `MetaCubeX/mihomo`
pub struct Request(String);
impl Request {
    const CLASH: &str = "MetaCubeX/mihomo";
    const CLASHTUI: &str = "JohanChane/clashtui";
    pub fn as_url(&self) -> String {
        format!("https://api.github.com/repos/{}/releases/latest", self.0)
    }
    /// a shortcut for `Request(Request::MIHOMO.to_owned())`
    pub fn s_clash() -> Self {
        Self(Self::CLASH.to_owned())
    }
    /// a shortcut for `Request(Request::CLASHTUI.to_owned())`
    pub fn s_clashtui() -> Self {
        Self(Self::CLASHTUI.to_owned())
    }
}

#[cfg_attr(test, derive(Default))]
#[derive(Debug, Deserialize)]
pub struct Response {
    pub name: String,
    pub tag_name: String,
    // pub draft: bool,
    // pub prerelease: bool,
    pub body: String,

    pub published_at: String,
    pub assets: Vec<Asset>,
}
impl Response {
    pub fn is_newer_than(&self, other: &str) -> bool {
        if let (Some(origin), Some(other)) =
            (get_triple_code(&self.tag_name), get_triple_code(other))
        {
            origin[0]
                .cmp(&other[0])
                .then(origin[1].cmp(&other[1]))
                .then(origin[2].cmp(&other[2]))
                .is_gt()
        } else {
            false
        }
    }
    /// filter by name contains `os` and `arch`
    ///
    /// if fail to get `arch`, then apply `os` only
    ///
    /// e.g. `*-linux-*-amd64-*`
    pub fn filter_asserts(mut self) -> Self {
        self.assets = self
            .assets
            .into_iter()
            .filter(|a| a.name.contains(std::env::consts::OS))
            .filter(|a| a.name.contains(get_arch()))
            .collect();
        self
    }
    pub fn as_info(&self, version: String) -> String {
        format!(
            "There is a new update for `{}`\nCurrent installed version is {version}\nPublished at {}\n\n---\n\nCHANGELOG:\n{}",
            self.name,
            self.published_at,
            self.body.trim_end().trim_start()
        )
    }
}
#[derive(Debug, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}
fn get_triple_code(tag: &str) -> Option<Vec<u8>> {
    let triple_code: Vec<u8> = tag
        .trim_start_matches('v')
        .split('-')
        .filter(|s| s.len() != 0)
        .take(1)
        .map(|s| s.split('.'))
        .flatten()
        .map(|s| s.parse::<u8>().unwrap_or(0))
        .collect();
    if triple_code.len() == 3 {
        Some(triple_code)
    } else {
        log::error!("Failed to decode tag: {}", tag);
        None
    }
}
fn get_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86" => "386",
        "x86_64" => "amd64",
        // "arm" => "",
        "aarch64" => "arm64",
        _ => "",
    }
}
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn version_code() {
        let mut r = Response {
            tag_name: "v0.1.0-keep".to_owned(),
            ..Default::default()
        };
        assert!(!r.is_newer_than(crate::consts::VERSION));
        r.tag_name = crate::consts::VERSION.to_owned();
        assert!(!r.is_newer_than(crate::consts::VERSION))
    }
}
