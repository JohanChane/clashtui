use super::ClashUtil;
use crate::Result;

impl ClashUtil {
    pub fn mock_clash_core<S: Into<minreq::URL>>(
        &self,
        url: S,
        with_proxy: bool,
    ) -> Result<minreq::ResponseLazy> {
        let mut req = minreq::get(url);
        if with_proxy {
            req = req.with_proxy(minreq::Proxy::new(self.proxy_addr.clone())?)
        }
        req.with_header("user-agent", self.ua.as_deref().unwrap_or("clash.meta"))
            .with_timeout(self.timeout)
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
