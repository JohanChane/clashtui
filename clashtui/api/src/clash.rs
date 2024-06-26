use minreq::Method;

const DEFAULT_PAYLOAD: &str = r#"'{"path": "", "payload": ""}'"#;
const TIMEOUT: u64 = 5;
#[cfg(feature = "deprecated")]
const GEO_URI: &str = "https://api.github.com/repos/MetaCubeX/meta-rules-dat/releases/latest";
#[cfg(feature = "deprecated")]
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

type Result<T> = core::result::Result<T, String>;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum ProfileSectionType {
    Profile,
    ProxyProvider,
    RuleProvider,
}

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
    #[cfg(target_feature = "deprecated")]
    pub fn check_geo_update(
        &self,
        old_id: Option<&String>,
        path: &std::path::Path,
    ) -> Result<String> {
        let info = self
            .client
            .get_or_init(Client::new)
            .get(GEO_URI)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .map_err(process_err)?
            .text()
            .map_err(process_err)?;
        let info: crate::geo::GithubApi =
            serde_json::from_str(info.as_str()).map_err(Error::from)?;
        if info.check(old_id) {
            let (assets, publist_at) = info.into();
            let result: Vec<Error> = assets
                .into_iter()
                // .inspect(|e| println!("{e:?}"))
                .map(|info| info.into())
                // ignore sha check for now, though no future plan to add
                .filter(|(name, _)| !name.ends_with("sha256sum"))
                .filter_map(|(name, url)| {
                    let path = path.join(name);
                    self.client
                        .get_or_init(Client::new)
                        .get(url)
                        .send()
                        .map_err(process_err)
                        .and_then(|dow| {
                            std::fs::File::options()
                                .create(true)
                                .write(true)
                                .open(path)
                                .and_then(|mut file| {
                                    file.write_all(dow.text().map_err(process_err)?.as_bytes())
                                })
                        })
                        .err()
                })
                // .inspect(|e| println!("{e:?}"))
                .collect();
            if result.is_empty() {
                Ok(publist_at)
            } else {
                Err(Error::new(ErrorKind::Other, format!("{result:?}")))
            }
        } else {
            Ok("Already Up to dated".to_string())
        }
    }

    /*
    pub fn flush_fakeip(&self) -> Result<String, reqwest::Error> {
        self.post("/cache/fakeip/flush", None)
    }
    pub fn provider(&self, is_rule: bool, name:Option<&String>, is_update: bool, is_check: bool) -> Result<String, reqwest::Error>{
        //
        if !is_rule{
            let api = "/providers/proxies";
            match name {
                Some(v) => {
                    if is_update{
                        self.put(&format!("{}/{}", api, v), None)
                    } else {
                        if is_check{
                            self.get(&format!("{}/{}/healthcheck", api, v), None)
                        } else {
                            self.get(&format!("{}/{}", api, v), None)
                        }
                    }
                },
                None => self.get(api, None),
            }
        } else {
            let api = "/providers/rules";
            match name {
                Some(v) => self.put(&format!("{}/{}", api, v), None),
                None => self.get(api, None)
            }
        }
    }
    pub fn update_geo(&self, payload:Option<&String>) -> Result<String, reqwest::Error>{
        match payload {
            Some(load) => self.post("/configs/geo", Some(load)),
            None => self.post("/configs/geo", Some(&self.default_payload))
        }
    }
    pub fn log(&self) -> Result<String, reqwest::Error>{
        self.get("/logs", None)
    }
    pub fn traffic(&self) -> Result<String, reqwest::Error>{
        self.get(&"/traffic", None)
    }
    pub fn memory(&self) -> Result<String, reqwest::Error>{
        self.get(&"/memory", None)
    }
    pub fn upgrade(&self, payload:Option<&String>) -> Result<String, reqwest::Error>{
        match payload {
            Some(load) => self.post("/upgrade", Some(load)),
            None => self.post("/upgrade", Some(&self.default_payload))
        }
    }
    pub fn upgrade_ui(&self) -> Result<String, reqwest::Error>{
        self.post("/upgrade/ui", None)
    }
    pub fn proxies(&self, name:Option<&String>, test_delay: bool) -> Result<String, reqwest::Error>{
        let api = match name {
            Some(v) => if test_delay {
                format!("/proxies/{}/delay", v)
            } else {
                format!("/proxies/{}", v)
            },
            None => "/proxies/".to_string()
        };
        self.get(&api, None)
    }
    pub fn set_proxy(&self, name:&String) -> Result<String, reqwest::Error> {
        self.put(&format!("/proxies/{}", name), None)
    }
    pub fn rules(&self) -> Result<String, reqwest::Error>{
        self.get("/rules", None)
    }
    pub fn connection(&self, is_close: bool, id:Option<usize>) -> Result<String, reqwest::Error>{
        if !is_close {
            self.get("/connections", None)
        } else {
            match
                match id {
                    Some(v) => self.client.delete(format!("/connections/{}", v)).send(),
                    None => self.client.delete("/connections").send(),
                }
                {
                Ok(r) => r.text(),
                Err(e) => {
                    log::error!("[ClashUtil] {} exec {} failed! {}", "patch", "/configs", e.status().unwrap());
                    Err(e)
                }
            }
        }
    }
    pub fn dns_resolve(&self, name:&String, _type:Option<&String>) -> Result<String, reqwest::Error>{
        match _type {
            Some(v) => self.get(&format!("/dns/query?name={}&type={}", name, v), None),
            None => self.get(&format!("/dns/query?name={}", name), None),
        }
    }
    */
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
