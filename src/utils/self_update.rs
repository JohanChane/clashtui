use serde::Deserialize;

use crate::clash::{headers, net_file::get_blob};

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
    /// Fetch info from given input
    ///
    /// support Github only
    pub fn get_info(self) -> anyhow::Result<Response> {
        let rdr = get_blob(self.as_url(), None, Some(headers::DEFAULT_USER_AGENT))?;
        Ok(serde_json::from_reader(rdr)?)
    }
}
impl Request<'static> {
    const MIHOMO: &str = "MetaCubeX/mihomo";
    const MIHOMO_CI: &str = "Prerelease-Alpha";
    const CLASHTUI: &str = "JohanChane/clashtui";
    const CLASHTUI_CI: &str = "Continuous_Integration";
    pub fn s_mihomo(is_ci: bool) -> Self {
        if is_ci {
            Self::WithTag(Self::MIHOMO, Self::MIHOMO_CI)
        } else {
            Self::Latest(Self::MIHOMO)
        }
    }
    pub fn s_clashtui(is_ci: bool) -> Self {
        if is_ci {
            // main repo hasn't start CI release yet
            Self::WithTag("Jackhr-arch/clashtui", Self::CLASHTUI_CI)
        } else {
            Self::Latest(Self::CLASHTUI)
        }
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
    /// wrapper for `is_newer_than`
    pub fn check(self, version: &str, skip_check: bool) -> Option<Self> {
        if skip_check || self.is_newer_than(version) {
            Some(self)
        } else {
            None
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
    pub fn rename(self, name: &str) -> Self {
        Self {
            name: name.to_owned(),
            ..self
        }
    }
    pub fn as_info(&self, version: String) -> String {
        format!(
            r#"----------------------
There is a new update for '{}'
Target version is '{}'
> Current version is '{version}'
Published at {}

CHANGELOG:
{}
"#,
            self.name,
            self.tag_name,
            self.published_at,
            self.body.trim()
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
        write!(f, "{}", self.name)
    }
}
impl Asset {
    pub fn download(&self, path: &std::path::Path) -> anyhow::Result<()> {
        download_to_file(path, &self.browser_download_url)
    }
}

pub fn download_to_file(path: &std::path::Path, url: &str) -> anyhow::Result<()> {
    match get_blob(url, None, Some(headers::DEFAULT_USER_AGENT)) {
        Ok(mut rp) => {
            let mut fp = std::fs::File::create(path)?;
            std::io::copy(&mut rp, &mut fp)?;
        }
        Err(e) => {
            eprintln!("{e}, try to download with `curl/wget`");
            use std::process::{Command, Stdio};
            fn have_this_and_exec(this: &str, args: &[&str]) -> anyhow::Result<bool> {
                if Command::new("which")
                    .arg(this)
                    .output()
                    .is_ok_and(|r| r.status.success())
                {
                    println!("using {this}");
                    if Command::new(this)
                        .args(args)
                        .stdin(Stdio::null())
                        .status()?
                        .success()
                    {
                        Ok(true)
                    } else {
                        Err(anyhow::anyhow!("Failed to download with {this}"))
                    }
                } else {
                    Ok(false)
                }
            }
            if !have_this_and_exec("curl", &["-o", path.to_str().unwrap(), "-L", url])?
                && !have_this_and_exec("wget", &["-O", path.to_str().unwrap(), url])?
            {
                anyhow::bail!("Unable to find curl/wget")
            }
        }
    }
    Ok(())
}

/// actually match `v0.12.432-`
fn get_triple_code(tag: &str) -> Option<[u16; 3]> {
    let triple_code: Vec<u16> = tag
        .trim_start_matches('v')
        .split('-')
        .filter(|s| !s.is_empty())
        .take(1)
        .flat_map(|s| s.split('.'))
        .map(|s| s.parse().unwrap_or(0))
        .collect();
    if triple_code.len() == 3 {
        // Some(triple_code)
        Some([triple_code[0], triple_code[1], triple_code[2]])
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
        let raw = "!Latest MetaCubeX/mihomo";
        assert_eq!(
            serde_yml::from_str::<Request>(raw).unwrap(),
            Request::s_mihomo(false)
        );
        let raw = "!WithTag\n- MetaCubeX/mihomo\n- Prerelease-Alpha";
        assert_eq!(
            serde_yml::from_str::<Request>(raw).unwrap(),
            Request::s_mihomo(true)
        );
        let raw = r#"
- !WithTag
  - Jackhr-arch/clashtui
  - Continuous_Integration
- !Latest JohanChane/clashtui
"#;
        assert_eq!(
            serde_yml::from_str::<Vec<Request>>(raw).unwrap(),
            vec![Request::s_clashtui(true), Request::s_clashtui(false)]
        );
    }
}
