use std::{
    cell::RefCell,
    io::{Error, ErrorKind},
};

const DEFAULT_PAYLOAD: &str = "'{\"path\": \"\", \"payload\": \"\"}'";

fn process_err(e: reqwest::Error) -> Error {
    if e.is_connect() {
        Error::new(ErrorKind::ConnectionRefused, e)
    } else if e.is_timeout() {
        Error::new(ErrorKind::TimedOut, e)
    } else {
        Error::new(ErrorKind::Other, e)
    }
}
pub struct Resp {
    inner: reqwest::blocking::Response,
}
impl Resp {
    pub fn copy_to<W: ?Sized>(self, w: &mut W) -> Result<u64, Error>
    where
        W: std::io::Write,
    {
        let Resp { mut inner } = self;
        std::io::copy(&mut inner, w)
    }
}
pub struct ClashUtil {
    client: reqwest::blocking::Client,
    api: String,
    proxy_addr: String,
    clash_client: RefCell<Option<reqwest::blocking::Client>>,
}

impl ClashUtil {
    pub fn new(controller_api: String, proxy_addr: String) -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            api: controller_api,
            proxy_addr,
            clash_client: None.into(),
        }
    }
    fn get(&self, url: &str, payload: Option<String>) -> Result<String, reqwest::Error> {
        let api = format!("{}{}", self.api, url);
        let response = match payload {
            Some(kv) => self.client.get(format!("{}{}", api, kv)),
            None => self.client.get(api),
        }
        .send();
        match response {
            Ok(r) => r.text(),
            Err(e) => Err(e),
        }
    }
    fn post(&self, url: &str, payload: Option<String>) -> Result<String, reqwest::Error> {
        let api = format!("{}{}", self.api, url);
        let response = match payload {
            Some(kv) => self.client.post(api).body(kv),
            None => self.client.post(api),
        }
        .send();
        match response {
            Ok(r) => r.text(),
            Err(e) => Err(e),
        }
    }

    fn put(&self, url: &str, payload: Option<String>) -> Result<String, reqwest::Error> {
        let api = format!("{}{}", self.api, url);
        let response = match payload {
            Some(kv) => self.client.put(api).body(kv),
            None => self.client.put(api),
        }
        .send();
        match response {
            Ok(r) => r.text(),
            Err(e) => Err(e),
        }
    }

    pub fn restart(&self, payload: Option<String>) -> Result<String, std::io::Error> {
        match payload {
            Some(load) => self.post("/restart", Some(load)),
            None => self.post("/restart", Some(DEFAULT_PAYLOAD.to_string())),
        }
        .map_err(process_err)
    }
    pub fn version(&self) -> Result<String, std::io::Error> {
        self.get("/version", None).map_err(process_err)
    }
    pub fn config_get(&self) -> Result<String, Error> {
        self.get("/configs", None).map_err(process_err)
    }
    pub fn config_reload(&self, payload: String) -> Result<(), Error> {
        match self.put("/configs?force=true", Some(payload)) {
            Err(e) => Err(process_err(e)),
            _ => Ok(()),
        }
    }
    pub fn mock_clash_core(&self, url: &str) -> Result<Resp, Error> {
        let mut hmap = self.clash_client.borrow_mut();
        if hmap.is_none() {
            let proxy = reqwest::Proxy::http(&self.proxy_addr).map_err(process_err)?;
            let client = reqwest::blocking::Client::builder()
                .user_agent("clash.meta")
                .proxy(proxy)
                .build()
                .map_err(process_err)?;
            hmap.replace(client);
        }
        hmap.as_ref()
            .unwrap()
            .get(url)
            .send()
            .map(|v| Resp { inner: v })
            .map_err(process_err)
    }
    pub fn config_patch(&self, payload: String) -> Result<String, Error> {
        self.client
            .patch(self.api.clone() + "/configs")
            .body(payload)
            .send()
            .map_err(process_err)?
            .text()
            .map_err(process_err)
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

    #[test]
    fn test() {
        let mut is = true;
        let sym = ClashUtil::new(
            "http://127.0.0.1:9090".to_string(),
            "http://127.0.0.1:7890".to_string(),
        );
        match sym.version() {
            Ok(r) => println!("{:?}", r),
            Err(_) => is = false,
        }
        assert!(is)
    }

    #[test]
    fn test_connection() {
        let c = ClashUtil::new(
            "http://127.0.0.1:9090".to_string(),
            "http://127.0.0.1:7890".into(),
        );
        let res = c.get("", None);
        println!("{:?}", res);
    }
}
