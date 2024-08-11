const DEFAULT_PAYLOAD: &str = "'{\"path\": \"\", \"payload\": \"\"}'";
const TIMEOUT: u8 = 10;
#[cfg(target_feature = "deprecated")]
const GEO_URI: &str = "https://api.github.com/repos/MetaCubeX/meta-rules-dat/releases/latest";
#[cfg(target_feature = "deprecated")]
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

use minreq::Method;
use std::io::Result;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum UrlType {
    Generic,
    GitHub,
    Gitee,
    Unknown
}

impl UrlType {
    fn from_str(s: &str) -> Self {
        match s {
            "github" => UrlType::GitHub,
            "gitee" => UrlType::Gitee,
            _ => UrlType::Unknown,
        }
    }
}

// {type: github|gitee, url, with_proxy}
pub struct UrlItem {
    pub typ: UrlType,
    pub url: String,
    pub token: Option<String>,
    pub with_proxy: bool
}

impl UrlItem {
    pub fn new(typ: UrlType, url: String, token: Option<String>, with_proxy: bool) -> Self {
        Self {
            typ: typ,
            url: url,
            token: token,
            with_proxy: with_proxy
        }
    }

    pub fn from_yaml(url_value: &serde_yaml::Value) -> Self {
        let typ = url_value.get("type")
            .and_then(serde_yaml::Value::as_str)
            .map(UrlType::from_str)
            .unwrap_or(UrlType::Unknown);

        let url = url_value.get("url")
            .and_then(serde_yaml::Value::as_str)
            .unwrap_or("")
            .to_string();

        let token = url_value.get("token")
            .and_then(serde_yaml::Value::as_str)
            .map(String::from);

        let with_proxy = url_value.get("with_proxy")
            .and_then(serde_yaml::Value::as_bool)
            .unwrap_or(true);

        UrlItem::new(typ, url, token, with_proxy)
    }

    pub fn gen_url(&self) -> Option<String> {
        if self.token.is_none() {
            return Some(self.url.clone());
        }

        // url format: 'https://x-access-token:token@url_without_protocol'
        // e.g. 'https://x-access-token:<token>@raw.githubusercontent.com/<User>/<Repo>/<Branch>/config.yaml'
        if self.typ == UrlType::GitHub {
            let token_part = if let Some(token) = &self.token {
                format!("x-access-token:{}", token)
            } else {
                String::new()
            };

            let url_without_protocol = self.url.trim_start_matches("https://");

            let url_with_auth = if !token_part.is_empty() {
                format!("{}@{}", token_part, url_without_protocol)
            } else {
                url_without_protocol.to_string()
            };

            return Some(format!("https://{}", url_with_auth));
        }

        None
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum ProfileSectionType {
    Profile,
    ProxyProvider,
    RuleProvider,
}

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

    pub fn get_headers(&self) -> &std::collections::HashMap<String, String> {
        &self.0.headers
    }

    pub fn to_json(self) -> Result<serde_json::Value> {
        let Resp(inner) = self;
        let body = serde_json::from_reader(inner)?;
        Ok(body)
    }
}
pub struct ClashUtil {
    pub api: String,
    secret: String,
    pub proxy_addr: String,
    pub clash_ua: String,
}

impl ClashUtil {
    pub fn new(
        controller_api: String,
        secret: String,
        proxy_addr: String,
        clash_ua: String,
    ) -> Self {
        Self {
            api: controller_api,
            secret,
            proxy_addr,
            clash_ua,
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
        if !self.secret.is_empty() {
            req = req.with_header("Authorization", format!("Bearer {0}", self.secret));
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

    // TODO: get blob path
    // Some REST API use sha in blob path. e.g.
    // 'https://gitee.com/api/v5/swagger#/getV5ReposOwnerRepoGitBlobsSha',
    // 'https://docs.gitcode.com/docs/openapi/repos/'
    pub fn get_blob_path(
        &self,
        url_item: &UrlItem,
        timeout: u8,
    ) -> Result<String> {
        let mut request = self.make_req(url_item, timeout);
        // TODO: with header

        let rsp = self.mock_clash_core(url_item, timeout)?;
        let file_info = rsp.to_json()?;
        let sha = file_info.get("sha").ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "No sha found"))?;
        // https://api.gitcode.com/api/v5/repos/{owner}/{repo}/contents/{path}
        // https://api.gitcode.com/api/v5/repos/{owner}/{repo}/git/blobs/{sha}
        Ok(format!("https://api.gitcode.com/api/v5/repos/{}/{}/git/blobs/{}", "User", "Branch", sha))
    }

    pub fn make_req(
        &self,
        url_item: &UrlItem,
        timeout: u8,
    ) -> Result<minreq::Request> {
        let mut request = minreq::get(url_item.url.as_str());

        if url_item.typ == UrlType::GitHub {
        } else if url_item.typ == UrlType::Gitee {
        }
        match url_item.typ {
            UrlType::GitHub => {
                if url_item.token.is_some() {
                    request = request.with_header("Authorization", format!("token {}", url_item.token.as_ref().unwrap()));
                }
            }
            UrlType::Gitee => {
                if url_item.token.is_some() {
                    request = request.with_header("Authorization", format!("Bearer {}", url_item.token.as_ref().unwrap()));
                    request = request.with_header("Accept", "application/vnd.github.v3.raw");
                }
            }
            _ => {}
        }

        if timeout > 0 {
            request = request.with_timeout(timeout.into());
        }

        if url_item.with_proxy {
            request = request
                .with_proxy(minreq::Proxy::new(self.proxy_addr.clone()).map_err(process_err)?);
        }

        Ok(request)
    }

    pub fn mock_clash_core(
        &self,
        url_item: &UrlItem,
        timeout: u8,
    ) -> Result<Resp> {
        let mut request = self.make_req(url_item, timeout)?;
        request = request.with_header("user-agent", self.clash_ua.clone());

        request.send_lazy().map(Resp).map_err(process_err)
    }
    pub fn config_patch(&self, payload: String) -> Result<String> {
        self.request(Method::Patch, "/configs", Some(payload))
    }

    pub fn check_connectivity(&self) -> Result<()> {
        minreq::get("https://www.gstatic.com/generate_204")
            .with_timeout(TIMEOUT.into())
            .with_proxy(minreq::Proxy::new(self.proxy_addr.clone()).map_err(process_err)?)
            .send()
            .map(|_| ())
            .map_err(process_err)
    }

    pub fn close_connnections(&self) -> Result<String> {
        self.request(Method::Delete, "/connections", None)
    }

    /*** update_providers_with_api
    pub fn update_providers(&self, provider_type: ProfileSectionType) -> Result<Vec<(String, Result<String>)>> {
        self.extract_net_providers(provider_type).and_then(|names| self.update_providers_helper(names, provider_type))
    }

    pub fn update_providers_helper(&self, provider_names: Vec<String>, provider_type: ProfileSectionType) -> Result<Vec<(String, Result<String>)>> {
        let mut result = Vec::<(String, Result<String>)>::new();
        for name in provider_names {
            let sub_url = format!("/providers/{}/{}", provider_str_in_api(provider_type).unwrap(), name);
            result.push((name, self.request(Method::Put, sub_url.as_str(), None)));
        }
        Ok(result)
    }

    pub fn extract_net_providers(&self, provider_type: ProfileSectionType) -> Result<Vec<String>>{
        let sub_url = format!("/providers/{}", provider_str_in_api(provider_type).unwrap());
        let response_str = self.request(Method::Get, sub_url.as_str(), None)?;

        let json_data: serde_json::Value = serde_json::from_str(response_str.as_str())?;
        let mut net_providers = Vec::new();
        if let Some(providers) = json_data["providers"].as_object() {
            let provider_type_str = match provider_type {
                ProfileSectionType::ProxyProvider => Some("Proxy"),
                ProfileSectionType::RuleProvider => Some("Rule"),
                _ => None,
            };
            for (_, provider) in providers.iter() {
                if let (Some(p_name), Some(p_type), Some(vehicle_type)) = (
                    provider.get("name").and_then(serde_json::Value::as_str),
                    provider.get("type").and_then(serde_json::Value::as_str),
                    provider.get("vehicleType").and_then(serde_json::Value::as_str),
                ) {
                    if vehicle_type == "HTTP" {
                        if Some(p_type) == provider_type_str {
                            net_providers.push(p_name.to_string());
                        }
                    }
                }
            }
        }

        Ok(net_providers)
    }

    // Sometime mihomo updated the provider but not update it to the file.
    pub fn extract_provider_utimes_with_api(&self, provider_type: ProfileSectionType) -> Result<Vec<(String, Option<std::time::SystemTime>)>>{
        use chrono::{DateTime, Local};

        let sub_url = format!("/providers/{}", provider_str_in_api(provider_type).unwrap());
        let response_str = self.request(Method::Get, sub_url.as_str(), None)?;

        let json_data: serde_json::Value = serde_json::from_str(response_str.as_str())?;
        let mut net_providers = Vec::new();
        if let Some(providers) = json_data["providers"].as_object() {
            let provider_type_str = match provider_type {
                ProfileSectionType::ProxyProvider => Some("Proxy"),
                ProfileSectionType::RuleProvider => Some("Rule"),
                _ => None,
            };
            for (_, provider) in providers.iter() {
                if let (Some(p_name), Some(p_type), Some(vehicle_type), Some(time_str)) = (
                    provider.get("name").and_then(serde_json::Value::as_str),
                    provider.get("type").and_then(serde_json::Value::as_str),
                    provider.get("vehicleType").and_then(serde_json::Value::as_str),
                    provider.get("updatedAt").and_then(serde_json::Value::as_str),
                ) {
                    if vehicle_type == "HTTP" {
                        if Some(p_type) == provider_type_str {
                                let parsed_time = DateTime::parse_from_rfc3339(time_str);
                                net_providers.push((
                                    p_name.to_string(),
                                    parsed_time.ok().map(|t| t.with_timezone(&Local).into())
                                ));
                        }
                    }
                }
            }
        }

        Ok(net_providers)
    }

    pub fn provider_str_in_api(section_type: ProfileSectionType) -> Option<String> {
        match section_type {
            ProfileSectionType::ProxyProvider => Some("proxies".to_string()),
            ProfileSectionType::RuleProvider => Some("rules".to_string()),
            _ => None,
        }
    }
    ***/

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
    use super::{ClashUtil, ProfileSectionType, UrlItem, UrlType};
    fn sym() -> ClashUtil {
        ClashUtil::new(
            "http://127.0.0.1:9090".to_string(),
            "test".to_string(),
            "http://127.0.0.1:7890".to_string(),
            "clash.meta".to_string(),
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
        let url_item = UrlItem::new(UrlType::Generic, "https://www.google.com".to_string(), None, true);
        let r = sym
            .mock_clash_core(&url_item, 10)
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
