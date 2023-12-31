pub struct ClashUtil{
    client: reqwest::blocking::Client,
    api: String,
    proxy_addr: String,
    default_payload: String,
}

impl ClashUtil {
    pub fn new(controller_api:String, proxy_addr:String) -> Self{
        let default_payload = "'{\"path\": \"\", \"payload\": \"\"}'".to_string();

    Self {
        client: reqwest::blocking::Client::new(),
        api: controller_api,
        proxy_addr,
        default_payload,
    }
    }
    fn get(&self, url:&str, payload:Option<&String>) -> Result<String, reqwest::Error>{
        let api = format!("{}{}", self.api, url);
        let response = match payload {
            Some(kv) => self.client.get(format!("{}{}", api, kv)),
            None => self.client.get(api),
        }.send();
        match response {
            Ok(r) => r.text(),
            Err(e) => {
                log::error!("[ClashUtil] exec {} failed! {}", url, e.status().unwrap());
                Err(e)
            }
        }
    }
    fn post(&self, url:&str, payload:Option<&String>) -> Result<String, reqwest::Error>{
        let api = format!("{}{}", self.api, url);
        let response = match payload {
            Some(kv) => self.client.post(api).body(kv.to_owned()),
            None => self.client.post(api),
        }.send();
        match response {
            Ok(r) => r.text(),
            Err(e) => {
                log::error!("[ClashUtil] exec {} failed! {}", url, e.status().unwrap());
                Err(e)
            }
        }
    }
    fn put(&self, url:&str, payload:Option<&String>) -> Result<String, reqwest::Error>{
        let api = format!("{}{}", self.api, url);
        let response = match payload {
            Some(kv) => self.client.put(api).body(kv.to_owned()),
            None => self.client.put(api),
        }.send();
        match response {
            Ok(r) => r.text(),
            Err(e) => {
                log::error!("[ClashUtil] exec {} failed! {}", url, e.status().unwrap());
                Err(e)
            }
        }
    }

    pub fn restart(&self) -> Result<String, reqwest::Error>{
        self.post("/restart", Some(&self.default_payload))
    }
    pub fn update_geo(&self) -> Result<String, reqwest::Error>{
        self.post("/configs/geo", Some(&self.default_payload))
    }
    pub fn log(&self) -> Result<String, reqwest::Error>{
        self.get("/logs", None)
    }
    pub fn traffic(&self) -> Result<String, reqwest::Error>{
        self.get(&"/traffic", None)
    }
    pub fn config_reload(&self, payload:String) -> Result<reqwest::blocking::Response, reqwest::Error>{
        let request = self.client
          .put(format!("{}/configs?force=true", self.api))
          .body(payload)
          .send();
    request
    }
    pub fn config_get(&self) -> Result<String, reqwest::Error>{
        self.get(&"/configs", None)
    }
    pub fn mock_clash_core(&self, url:&str) -> Result<reqwest::blocking::Response, reqwest::Error>{
        let proxy = reqwest::Proxy::http(&self.proxy_addr).unwrap();
        let clash_client = reqwest::blocking::Client::builder()
        //.user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/112.0.0.0 Safari/537.36 uacq")
        //.user_agent("clash-verge/v1.2.0") // url 后不加 `flag=clash` 也会返回 yaml 配置, 而不是返回 base64 编码。
        .user_agent("clash.meta")
        .proxy(proxy)
        .build()
        .unwrap();
        let response = clash_client.get(url).send();
    response
    }
}

#[test]
fn test(){
    let mut is = true;
    let sym = ClashUtil::new("http://127.0.0.1:9090".to_string(), "http://127.0.0.1:7890".to_string());
    match sym.config_get() {
        Ok(r) => println!("{:?}", r),
        Err(_) => is = false       
    }
    match sym.restart() {
        Ok(r) => println!("{:?}", r),
        Err(_) => is = false       
    }
    assert!(is)
}