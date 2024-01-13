use std::{cell::RefCell, collections::HashMap};

pub struct ClashUtil {
    client: reqwest::blocking::Client,
    api: String,
    proxy_addr: String,
    default_payload: String,
    clash_client: RefCell<HashMap<bool, reqwest::blocking::Client>>,
}

impl ClashUtil {
    pub fn new(controller_api: String, proxy_addr: String) -> Self {
        let default_payload = "'{\"path\": \"\", \"payload\": \"\"}'".to_string();

        Self {
            client: reqwest::blocking::Client::new(),
            api: controller_api,
            proxy_addr,
            default_payload,
            clash_client: HashMap::new().into(),
        }
    }
    fn get(&self, url: &str, payload: Option<&String>) -> Result<String, reqwest::Error> {
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
    fn post(&self, url: &str, payload: Option<&String>) -> Result<String, reqwest::Error> {
        let api = format!("{}{}", self.api, url);
        let response = match payload {
            Some(kv) => self.client.post(api).body(kv.to_owned()),
            None => self.client.post(api),
        }
        .send();
        match response {
            Ok(r) => r.text(),
            Err(e) => Err(e),
        }
    }

    fn put(&self, url: &str, payload: Option<&String>) -> Result<String, reqwest::Error> {
        let api = format!("{}{}", self.api, url);
        let response = match payload {
            Some(kv) => self.client.put(api).body(kv.to_owned()),
            None => self.client.put(api),
        }
        .send();
        match response {
            Ok(r) => r.text(),
            Err(e) => Err(e),
        }
    }

    pub fn restart(&self, payload: Option<&String>) -> Result<String, reqwest::Error> {
        match payload {
            Some(load) => self.post("/restart", Some(load)),
            None => self.post("/restart", Some(&self.default_payload)),
        }
    }
    #[allow(unused)]
    pub fn flush_fakeip(&self) -> Result<String, reqwest::Error> {
        self.post("/cache/fakeip/flush", None)
    }
    #[allow(unused)]
    pub fn version(&self) -> Result<String, reqwest::Error> {
        self.get(&"/version", None)
    }
    pub fn config_get(&self) -> Result<String, reqwest::Error> {
        self.get(&"/configs", None)
    }
    pub fn config_reload(&self, payload: String) -> Option<reqwest::Error> {
        match self.put("/configs?force=true", Some(&payload)) {
            Err(e) => Some(e),
            _ => None,
        }
    }
    pub fn mock_clash_core(
        &self,
        url: &str,
    ) -> Result<reqwest::blocking::Response, reqwest::Error> {
        let response;
        let mut hmap = self.clash_client.borrow_mut();
        match hmap.get(&true) {
            // Not very good, but it does work. Maybe you can do it better
            Some(c) => response = c.get(url).send(),
            None => {
                let proxy = reqwest::Proxy::http(&self.proxy_addr).unwrap();
                let client = reqwest::blocking::Client::builder()
                    //.user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/112.0.0.0 Safari/537.36 uacq")
                    //.user_agent("clash-verge/v1.2.0") // url 后不加 `flag=clash` 也会返回 yaml 配置, 而不是返回 base64 编码。
                    .user_agent("clash.meta")
                    .proxy(proxy)
                    .build()
                    .unwrap();
                response = client.get(url).send();
                hmap.insert(true, client);
                // println!("!! init one");
            }
        }
        response
    }
    pub fn config_patch(&self, payload: String) -> Result<String, reqwest::Error> {
        match self
            .client
            .patch(self.api.clone() + "/configs")
            .body(payload)
            .send()
        {
            Ok(r) => r.text(),
            Err(e) => Err(e),
        }
    }
    /*
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
#[allow(unused)]
fn test_clash_mock() {
    let stru = ClashUtil::new(
        "http://127.0.0.1:9090".to_string(),
        "http://127.0.0.1:7890".to_string(),
    );
    let r1 = stru.mock_clash_core("");
    let r2 = stru.mock_clash_core("");
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
