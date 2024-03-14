use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
pub struct GithubApi {
    // Not caring about the value, just keep it as string
    #[serde(deserialize_with = "serde_this_or_that::as_string")]
    pub id: String,
    pub name: String,
    pub tag_name: String,
    draft: bool,
    prerelease: bool,

    pub published_at: String,
    pub assets: Vec<Asset>,
}
impl GithubApi {
    pub fn check(&self, current_version: &str) -> bool {
        current_version == self.id && !self.draft && !self.prerelease
    }
    pub fn get_url(&self, target: usize) -> Option<&String> {
        self.assets.get(target).map(|asset| asset.get_url())
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}
impl Asset {
    pub fn get_url(&self) -> &String {
        &self.browser_download_url
    }
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
