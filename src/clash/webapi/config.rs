use super::{ClashConfig, ClashUtil, CResult};
use minreq::Method;

impl ClashUtil {
    pub fn config_get(&self) -> anyhow::Result<ClashConfig> {
        Ok(serde_json::from_str(
            self.request(Method::Get, "/configs", None)?.as_str()?,
        )?)
    }
    pub fn config_reload<S: AsRef<str>>(&self, path: S) -> CResult<String> {
        self.request(
            Method::Put,
            "/configs?force=true",
            Some(
                serde_json::json!({
                    "path": path.as_ref(),
                    "payload": ""
                })
                .to_string(),
            ),
        )
        .and_then(|r| r.as_str().map(|s| s.to_owned()))
        .map_err(|e| e.into())
    }
    pub fn config_patch(&self, payload: String) -> CResult<String> {
        self.request(Method::Patch, "/configs", Some(payload))
            .and_then(|r| r.as_str().map(|s| s.to_owned()))
            .map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::ClashUtil;
    #[test]
    fn config_get_test() {
        let sym = ClashUtil::build_test();
        println!("{}", sym.config_get().unwrap().mode);
    }
    #[test]
    fn config_patch_test() {
        let sym = ClashUtil::build_test();
        println!("{}", sym.config_patch("".to_string()).unwrap());
    }
    #[test]
    fn config_reload_test() {
        let sym = ClashUtil::build_test();
        sym.config_reload(super::super::DEFAULT_PAYLOAD.to_string())
            .unwrap();
    }
}
