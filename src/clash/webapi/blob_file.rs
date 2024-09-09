use super::super::Result;
use super::ClashUtil;

impl ClashUtil {
    pub fn mock_clash_core<S: Into<minreq::URL>>(
        &self,
        url: S,
        with_proxy: bool,
    ) -> Result<minreq::ResponseLazy> {
        self.get_blob(
            url,
            with_proxy.then_some(self.proxy_addr.as_str()),
            Some(self.ua.as_deref().unwrap_or("clash.meta")),
        )
    }
    pub fn get_blob<U: Into<minreq::URL>, S1: AsRef<str>, S2: Into<String>>(
        &self,
        url: U,
        proxy: Option<S1>,
        ua: Option<S2>,
    ) -> Result<minreq::ResponseLazy> {
        let mut req = minreq::get(url);
        if let Some(proxy) = proxy {
            req = req.with_proxy(minreq::Proxy::new(proxy)?)
        }
        if let Some(ua) = ua {
            req = req.with_header("user-agent", ua)
        }
        req.with_timeout(self.timeout)
            .send_lazy()
            .map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::ClashUtil;
    #[test]
    fn mock_clash_core_test() {
        let sym = ClashUtil::build_test();
        let mut r = sym
            .mock_clash_core("https://www.google.com", sym.version().is_ok())
            .unwrap();
        let mut tf = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open("/tmp/test")
            .unwrap();
        std::io::copy(&mut r, &mut tf).unwrap();
        drop(tf);
        std::fs::remove_file("/tmp/test").unwrap();
    }
}
