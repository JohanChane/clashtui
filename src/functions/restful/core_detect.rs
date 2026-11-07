use serde::Deserialize;

use super::*;

/// Response from the `/version` endpoint.
///
/// Sing-box ≥1.13.x also returns `"meta": true` in its clash API emulation,
/// so we detect by the `version` string contents instead.
#[derive(Deserialize)]
struct VersionResponse {
    version: String,
}

/// A single identity check supplies both the detected core and its version.
pub struct CoreVersion {
    pub core: crate::config::CoreType,
    pub version: String,
}

pub fn fetch_version() -> Result<CoreVersion> {
    let response: VersionResponse = request(Method::Get, "/version", None)?.json()?;
    let core = if response.version.contains("sing-box") {
        crate::config::CoreType::Singbox
    } else {
        crate::config::CoreType::Mihomo
    };
    Ok(CoreVersion {
        core,
        version: response.version,
    })
}
