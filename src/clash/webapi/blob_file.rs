use super::{CResult, ClashUtil};

impl ClashUtil {
    pub fn mock_clash_core<U: Into<minreq::URL>>(
        &self,
        url: U,
        with_proxy: bool,
    ) -> CResult<minreq::ResponseLazy> {
        super::net_file::get_blob(
            url,
            with_proxy.then_some(self.proxy_addr.as_str()),
            Some(self.ua.as_deref().unwrap_or("clash.meta")),
        )
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
