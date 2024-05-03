const DEFAULT_PAYLOAD: &str = r#"'{"path": "", "payload": ""}'"#;
const TIMEOUT: u8 = 3;
#[cfg(feature = "deprecated")]
const GEO_URI: &str = "https://api.github.com/repos/MetaCubeX/meta-rules-dat/releases/latest";
#[cfg(feature = "deprecated")]
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

use std::io::Result;

use minreq::Method;
trait ResProcess {
    fn process(self) -> core::result::Result<String, minreq::Error>;
}
impl ResProcess for minreq::Response {
    fn process(self) -> core::result::Result<String, minreq::Error> {
        self.as_str().map(|s| s.to_owned())
    }
}

fn process_err(e: minreq::Error) -> std::io::Error {
    use std::io::{Error, ErrorKind};
    match e {
        minreq::Error::AddressNotFound | minreq::Error::PunycodeConversionFailed => {
            Error::new(ErrorKind::AddrNotAvailable, e)
        }
        minreq::Error::IoError(e) => e,
        minreq::Error::HeadersOverflow
        | minreq::Error::StatusLineOverflow
        | minreq::Error::InvalidUtf8InBody(_)
        | minreq::Error::InvalidUtf8InResponse
        | minreq::Error::MalformedChunkLength
        | minreq::Error::MalformedChunkEnd
        | minreq::Error::MalformedContentLength => Error::new(ErrorKind::InvalidData, e),
        minreq::Error::RedirectLocationMissing
        | minreq::Error::InfiniteRedirectionLoop
        | minreq::Error::TooManyRedirections => Error::new(ErrorKind::ConnectionAborted, e),
        minreq::Error::HttpsFeatureNotEnabled => unreachable!("https should already be enabled"),
        minreq::Error::PunycodeFeatureNotEnabled => panic!("{}", e),
        minreq::Error::RustlsCreateConnection(_) => Error::new(ErrorKind::ConnectionRefused, e),
        minreq::Error::BadProxy
        | minreq::Error::BadProxyCreds
        | minreq::Error::ProxyConnect
        | minreq::Error::InvalidProxyCreds => Error::new(ErrorKind::PermissionDenied, e),
        minreq::Error::Other(i) => Error::new(ErrorKind::Other, i),
    }
}

pub struct Resp(minreq::ResponseLazy);
impl Resp {
    pub fn copy_to<W: ?Sized>(self, w: &mut W) -> std::io::Result<u64>
    where
        W: std::io::Write,
    {
        let Resp(mut inner) = self;
        std::io::copy(&mut inner, w)
    }
}
pub struct ClashUtil {
    api: String,
    secret: Option<String>,
    ua: Option<String>,
    pub proxy_addr: String,
}

impl ClashUtil {
    pub fn new(
        controller_api: String,
        secret: Option<String>,
        proxy_addr: String,
        ua: Option<String>,
    ) -> Self {
        Self {
            api: controller_api,
            secret,
            ua,
            proxy_addr,
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
        req.with_timeout(TIMEOUT.into())
            .send()
            .and_then(|r| r.as_str().map(|s| s.to_owned()))
            .map_err(process_err)
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
            req = req.with_proxy(minreq::Proxy::new(self.proxy_addr.clone()).map_err(process_err)?)
        }
        req.with_header(
            "user-agent",
            self.ua.as_deref().unwrap_or("clash.meta"),
        )
        .with_timeout(TIMEOUT.into())
        .send_lazy()
        .map(Resp)
        .map_err(process_err)
    }
    pub fn config_patch(&self, payload: String) -> Result<String> {
        self.request(Method::Patch, "/configs", Some(payload))
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
        )
    }
    #[test]
    fn version_test() {
        let sym = sym();
        println!("{}", sym.version().unwrap());
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
        let r = sym.mock_clash_core("https://www.google.com", true).unwrap();
        let mut tf = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open("test")
            .unwrap();
        r.copy_to(&mut tf).unwrap();
        drop(tf);
        std::fs::remove_file("test").unwrap();
    }
    #[test]
    #[cfg(target_feature = "deprecated")]
    fn test_geo_update() {
        let mut flag = true;
        let sym = ClashUtil::new(
            "http://127.0.0.1:9090".to_string(),
            "http://127.0.0.1:7890".to_string(),
        );
        let exe_dir = std::env::current_dir().unwrap();
        println!("{exe_dir:?}");
        let path = exe_dir.join("tmp");
        if !path.is_dir() {
            std::fs::create_dir_all(&path).unwrap()
        }
        println!(
            "result:{}",
            match sym.check_geo_update(None, &path) {
                Ok(r) => r,
                Err(e) => {
                    flag = false;
                    format!("{e:?}")
                }
            }
        );
        assert!(flag)
    }
}
