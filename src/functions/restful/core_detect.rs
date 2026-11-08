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

/// One `GET /version`: the running core and its version string.
///
/// sing-box reports e.g. `"sing-box 1.13.11"`, mihomo reports e.g. `"v1.18.10"`.
/// `/version` is not rejected by the mismatch guard in `request()`.
pub fn probe() -> Result<(crate::config::CoreType, String)> {
    request(Method::Get, "/version", None).and_then(|r| {
        let v: VersionResponse = r.json()?;
        let core = if v.version.contains("sing-box") {
            crate::config::CoreType::Singbox
        } else {
            crate::config::CoreType::Mihomo
        };
        Ok((core, v.version))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_singbox_from_version_string() {
        let json = r#"{"meta": true, "premium": true, "version": "sing-box 1.13.11"}"#;
        let v: VersionResponse = serde_json::from_str(json).unwrap();
        assert!(v.version.contains("sing-box"));
    }

    #[test]
    fn detect_mihomo_from_version_string() {
        let json = r#"{"meta": true, "version": "v1.18.10"}"#;
        let v: VersionResponse = serde_json::from_str(json).unwrap();
        assert!(!v.version.contains("sing-box"));
    }
}
