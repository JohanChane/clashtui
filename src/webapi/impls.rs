use minreq::{Method, Response, ResponseLazy};

const DEFAULT_PAYLOAD: &str = r#"'{"path": "", "payload": ""}'"#;

type Result<T> = core::result::Result<T, minreq::Error>;

use super::{ClashConfig, ClashUtil};

impl ClashUtil {
    fn request(
        &self,
        method: minreq::Method,
        sub_url: &str,
        payload: Option<String>,
    ) -> Result<Response> {
        let mut req = minreq::Request::new(method, self.api.to_owned() + sub_url);
        if let Some(kv) = payload {
            req = req.with_body(kv);
        }
        if let Some(s) = self.secret.as_ref() {
            req = req.with_header("Authorization", format!("Bearer {s}"));
        }
        req.with_timeout(self.timeout).send()
        // .and_then(|r| r.as_str().map(|s| s.to_owned()))
    }
    pub fn restart(&self, payload: Option<String>) -> Result<String> {
        self.request(
            Method::Post,
            "/restart",
            Some(payload.unwrap_or(DEFAULT_PAYLOAD.to_string())),
        )
        .and_then(|r| r.as_str().map(|s| s.to_owned()))
    }
    pub fn version(&self) -> Result<String> {
        self.request(Method::Get, "/version", None)
            .and_then(|r| r.as_str().map(|s| s.to_owned()))
    }
    pub fn config_get(&self) -> anyhow::Result<ClashConfig> {
        Ok(serde_json::from_str(
            self.request(Method::Get, "/configs", None)?.as_str()?,
        )?)
    }
    pub fn config_reload<S: AsRef<str>>(&self, path: S) -> Result<String> {
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
    }
    pub fn mock_clash_core<S: Into<minreq::URL>>(
        &self,
        url: S,
        with_proxy: bool,
    ) -> Result<ResponseLazy> {
        let mut req = minreq::get(url);
        if with_proxy {
            req = req.with_proxy(minreq::Proxy::new(self.proxy_addr.clone())?)
        }
        req.with_header("user-agent", self.ua.as_deref().unwrap_or("clash.meta"))
            .with_timeout(self.timeout)
            .send_lazy()
    }
    pub fn config_patch(&self, payload: String) -> Result<String> {
        self.request(Method::Patch, "/configs", Some(payload))
            .and_then(|r| r.as_str().map(|s| s.to_owned()))
    }
    pub fn check_connectivity(&self) -> Result<()> {
        minreq::get("https://www.gstatic.com/generate_204")
            .with_timeout(self.timeout)
            .with_proxy(minreq::Proxy::new(self.proxy_addr.clone())?)
            .send()
            .map(|_| ())
    }
}
#[cfg(test)]
mod tests {
    use super::ClashUtil;
    fn sym() -> ClashUtil {
        ClashUtil::new(
            "http://127.0.0.1:9090".to_string(),
            None,
            "http://127.0.0.1:7890".to_string(),
            None,
            None,
        )
    }
    #[test]
    fn version_test() {
        let sym = sym();
        println!("{}", sym.version().unwrap());
    }
    #[test]
    #[should_panic]
    fn connectivity_test() {
        let sym = sym();
        println!("{:?}", sym.check_connectivity().unwrap_err());
    }
    #[test]
    fn config_get_test() {
        let sym = sym();
        println!("{}", sym.config_get().unwrap().mode);
    }
    #[test]
    fn config_patch_test() {
        let sym = sym();
        println!("{}", sym.config_patch("".to_string()).unwrap());
    }
    #[test]
    fn config_reload_test() {
        let sym = sym();
        sym.config_reload(super::DEFAULT_PAYLOAD.to_string())
            .unwrap();
    }
    #[test]
    fn mock_clash_core_test() {
        let sym = sym();
        let mut r = sym
            .mock_clash_core("https://www.google.com", sym.version().is_ok())
            .unwrap();
        let mut tf = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open("test")
            .unwrap();
        std::io::copy(&mut r, &mut tf).unwrap();
        drop(tf);
        std::fs::remove_file("test").unwrap();
    }
}
