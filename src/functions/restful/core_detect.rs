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

/// Detect which core is actually running by querying `/version`.
///
/// Checks the `version` field: sing-box reports e.g. `"sing-box 1.13.11"`,
/// mihomo reports e.g. `"v1.18.10"`.
pub fn detect_core_type() -> Result<crate::config::CoreType> {
    request(Method::Get, "/version", None).and_then(|r| {
        let v: VersionResponse = r.json()?;
        if v.version.contains("sing-box") {
            Ok(crate::config::CoreType::Singbox)
        } else {
            Ok(crate::config::CoreType::Mihomo)
        }
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
