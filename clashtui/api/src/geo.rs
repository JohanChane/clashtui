use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct GithubApi {
    // Not caring about the value, just keep it as string
    #[serde(deserialize_with = "serde_this_or_that::as_string")]
    id: String,
    draft: bool,
    prerelease: bool,

    published_at: String,
    assets: Vec<Asset>,
}
impl GithubApi {
    pub fn check(&self, old_id: Option<&String>) -> bool {
        match old_id {
            Some(old_id) => old_id == &self.id && !self.draft && !self.prerelease,
            None => true,
        }
    }
}
impl From<GithubApi> for (Vec<Asset>, String) {
    fn from(value: GithubApi) -> Self {
        let GithubApi {
            assets,
            published_at,
            ..
        } = value;
        (assets, published_at)
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Asset {
    pub name: String,
    pub browser_download_url: String,
}
impl From<Asset> for (String, String) {
    fn from(value: Asset) -> Self {
        let Asset {
            name,
            browser_download_url,
            ..
        } = value;
        (name, browser_download_url)
    }
}
