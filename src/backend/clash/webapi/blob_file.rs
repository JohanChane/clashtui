use super::{
    super::{MinreqResult, headers},
    ClashUtil,
};

impl ClashUtil {
    pub fn mock_clash_core<U: Into<minreq::URL>>(&self, url: U, with_proxy: bool) -> MinreqResult {
        super::get_blob(
            url,
            with_proxy.then_some(self.proxy_addr.as_str()),
            Some(self.ua.as_deref().unwrap_or("clash.meta")),
            self.timeout,
        )
    }
    pub fn dl_github<U: Into<minreq::URL>>(
        &self,
        url: U,
        with_proxy: bool,
        token: String,
    ) -> MinreqResult {
        let mut req = super::make_request_with_cred(url)?;
        if with_proxy {
            req = req.with_proxy(minreq::Proxy::new(&self.proxy_addr)?);
        }
        req = req.with_header(headers::AUTHORIZATION, format!("Bearer {token}"));
        req.with_timeout(self.timeout).send_lazy()
    }
    pub fn dl_gitlab<U: Into<minreq::URL>>(
        &self,
        url: U,
        with_proxy: bool,
        token: String,
    ) -> MinreqResult {
        let mut req = super::make_request_with_cred(url)?;
        if with_proxy {
            req = req.with_proxy(minreq::Proxy::new(&self.proxy_addr)?);
        }
        req = req.with_header("PRIVATE-TOKEN", token);
        req.with_timeout(self.timeout).send_lazy()
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
            .truncate(false)
            .open("/tmp/clashtui.test")
            .unwrap();
        std::io::copy(&mut r, &mut tf).unwrap();
        drop(tf);
        std::fs::remove_file("/tmp/clashtui.test").unwrap();
    }
}
