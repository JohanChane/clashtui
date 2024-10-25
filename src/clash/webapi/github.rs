use serde::Deserialize;

use super::{CResult, ClashUtil};

impl ClashUtil {
    /// Fetch info from given input
    ///
    /// support Github only
    pub fn get_github_info(&self, request: &Request) -> CResult<Response> {
        use super::headers;
        self.get_blob(request.as_url(), None, Some(headers::DEFAULT_USER_AGENT))
            .and_then(|r| serde_json::from_reader(r).map_err(|e| e.into()))
    }
    /// try GET raw data from given `url`
    ///
    /// return an object that impl [Read](std::io::Read)
    pub fn get_file(&self, url: &str) -> CResult<minreq::ResponseLazy> {
        use super::headers;
        self.get_blob(url, None, Some(headers::DEFAULT_USER_AGENT))
    }
}
#[cfg_attr(test, derive(Deserialize, Debug, PartialEq))]
/// Describe target repo and tag
pub enum Request<'a> {
    Latest(&'a str),
    WithTag(&'a str, &'a str),
}
impl Request<'_> {
    pub fn as_url(&self) -> String {
        match self {
            Request::Latest(repo) => format!("https://api.github.com/repos/{repo}/releases/latest"),
            Request::WithTag(repo, name) => {
                format!("https://api.github.com/repos/{repo}/releases/tags/{name}")
            }
        }
    }
}
impl Request<'static> {
    const MIHOMO: &str = "MetaCubeX/mihomo";
    const MIHOMO_CI: &str = "Prerelease-Alpha";
    const CLASHTUI: &str = "JohanChane/clashtui";
    const CLASHTUI_CI: &str = "Continuous_Integration";
    pub fn s_mihomo() -> Self {
        Self::Latest(Self::MIHOMO)
    }
    pub fn s_clashtui() -> Self {
        Self::Latest(Self::CLASHTUI)
    }
    pub fn s_clashtui_ci() -> Self {
        // main repo hasn't start CI release yet
        Self::WithTag("Jackhr-arch/clashtui", Self::CLASHTUI_CI)
    }
    /// Alpha version
    pub fn s_mihomo_ci() -> Self {
        Self::WithTag(Self::MIHOMO, Self::MIHOMO_CI)
    }
}

#[cfg_attr(test, derive(Default))]
#[derive(Debug, Deserialize)]
/// Github API return form
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
    /// compare the version code
    ///
    /// it should be like `v0.12.432-bear`,
    /// and only `v0.12.432-` will be matched
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
impl std::fmt::Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.name, self.browser_download_url)
    }
}
/// actually match `v0.12.432-`,
/// return a [Vec] of [u16], len = 3
fn get_triple_code(tag: &str) -> Option<Vec<u16>> {
    let triple_code: Vec<u16> = tag
        .trim_start_matches('v')
        .split('-')
        .filter(|s| !s.is_empty())
        .take(1)
        .flat_map(|s| s.split('.'))
        .map(|s| s.parse().unwrap_or(0))
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
        // "x86" => "386",
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
    #[test]
    fn load_request() {
        let raw = "!Latest JohanChane/clashtui";
        assert_eq!(
            serde_yml::from_str::<Request>(raw).unwrap(),
            Request::s_clashtui()
        );
        let raw = "!WithTag\n- Jackhr-arch/clashtui\n- Continuous_Integration";
        assert_eq!(
            serde_yml::from_str::<Request>(raw).unwrap(),
            Request::s_clashtui_ci()
        );
        let raw = r#"
- !WithTag
  - Jackhr-arch/clashtui
  - Continuous_Integration
- !Latest JohanChane/clashtui
"#;
        assert_eq!(
            serde_yml::from_str::<Vec<Request>>(raw).unwrap(),
            vec![Request::s_clashtui_ci(), Request::s_clashtui()]
        );
    }
}
