use minreq::Method;

const DEFAULT_PAYLOAD: &str = r#"'{"path": "", "payload": ""}'"#;
const TIMEOUT: u64 = 5;

type Result<T> = core::result::Result<T, String>;

pub fn build_payload<P: AsRef<str>>(path: P) -> String {
    serde_json::json!({
        "path": path.as_ref(),
        "payload": ""
    })
    .to_string()
}

pub struct Resp(minreq::ResponseLazy);
impl Resp {
    pub fn copy_to<W>(self, w: &mut W) -> std::io::Result<u64>
    where
        W: std::io::Write + ?Sized,
    {
        let Resp(mut inner) = self;
        std::io::copy(&mut inner, w)
    }
}
#[derive(Debug)]
pub struct ClashUtil {
    api: String,
    secret: Option<String>,
    ua: Option<String>,
    timeout: u64,
    pub proxy_addr: String,
}

impl ClashUtil {
    pub fn new(
        controller_api: String,
        secret: Option<String>,
        proxy_addr: String,
        ua: Option<String>,
        timeout: Option<u64>,
    ) -> Self {
        Self {
            api: controller_api,
            secret,
            ua,
            proxy_addr,
            timeout: timeout.unwrap_or(TIMEOUT),
        }
    }
    fn request(
        &self,
        method: minreq::Method,
        sub_url: &str,
        payload: Option<String>,
    ) -> Result<String> {
        let mut req = minreq::Request::new(method, self.api.to_owned() + sub_url);
        if let Some(kv) = payload {
            req = req.with_body(kv);
        }
        if let Some(s) = self.secret.as_ref() {
            req = req.with_header("Authorization", format!("Bearer {s}"));
        }
        req.with_timeout(self.timeout)
            .send()
            .and_then(|r| r.as_str().map(|s| s.to_owned()))
            .map_err(|e| format!("API:{e:?}"))
    }
    pub fn restart(&self, payload: Option<String>) -> Result<String> {
        self.request(
            Method::Post,
            "/restart",
            Some(payload.unwrap_or(DEFAULT_PAYLOAD.to_string())),
        )
    }
    pub fn version(&self) -> Result<String> {
        self.request(Method::Get, "/version", None)
    }
    pub fn config_get(&self) -> Result<String> {
        self.request(Method::Get, "/configs", None)
    }
    pub fn config_reload(&self, payload: String) -> Result<()> {
        self.request(Method::Put, "/configs?force=true", Some(payload))
            .map(|_| ())
    }
    pub fn mock_clash_core<S: Into<minreq::URL>>(&self, url: S, with_proxy: bool) -> Result<Resp> {
        let mut req = minreq::get(url);
        if with_proxy {
            req = req.with_proxy(
                minreq::Proxy::new(self.proxy_addr.clone())
                    .map_err(|e| format!("API(PROXY):{e:?}"))?,
            )
        }
        req.with_header("user-agent", self.ua.as_deref().unwrap_or("clash.meta"))
            .with_timeout(self.timeout)
            .send_lazy()
            .map(Resp)
            .map_err(|e| format!("API:{e:?}"))
    }
    pub fn config_patch(&self, payload: String) -> Result<String> {
        self.request(Method::Patch, "/configs", Some(payload))
    }
    pub fn check_connectivity(&self) -> Result<()> {
        minreq::get("https://www.gstatic.com/generate_204")
            .with_timeout(self.timeout)
            .with_proxy(
                minreq::Proxy::new(self.proxy_addr.clone())
                    .map_err(|e| format!("API(PROXY):{e:?}"))?,
            )
            .send()
            .map(|_| ())
            .map_err(|e| format!("API:{e:?}"))
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
        println!("{}", sym.config_get().unwrap());
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
        let r = sym
            .mock_clash_core("https://www.google.com", sym.version().is_ok())
            .unwrap();
        let mut tf = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open("test")
            .unwrap();
        r.copy_to(&mut tf).unwrap();
        drop(tf);
        std::fs::remove_file("test").unwrap();
    }
}
