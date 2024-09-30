use super::{ClashUtil, DEFAULT_PAYLOAD};
use crate::CResult;
use minreq::Method;

impl ClashUtil {
    /// Restart clash core via http
    /// 
    /// usually, an empty str is returned
    pub fn restart(&self, payload: Option<String>) -> CResult<String> {
        self.request(
            Method::Post,
            "/restart",
            Some(payload.unwrap_or(DEFAULT_PAYLOAD.to_string())),
        )
        .and_then(|r| r.as_str().map(|s| s.to_owned()))
        .map_err(|e| e.into())
    }
    /// Get clash core version
    /// 
    /// for mihomo, it's like `{"meta": true, "version": "v1.1.1"}`
    pub fn version(&self) -> CResult<String> {
        self.request(Method::Get, "/version", None)
            .and_then(|r| r.as_str().map(|s| s.to_owned()))
            .map_err(|e| e.into())
    }
    /// Try GET `https://www.gstatic.com/generate_204`
    /// 
    /// return nothing on success
    pub fn check_connectivity(&self) -> CResult<()> {
        minreq::get("https://www.gstatic.com/generate_204")
            .with_timeout(self.timeout)
            .with_proxy(minreq::Proxy::new(self.proxy_addr.clone())?)
            .send()
            .map(|_| ())
            .map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::ClashUtil;
    #[test]
    fn version_test() {
        let sym = ClashUtil::build_test();
        println!("{}", sym.version().unwrap());
    }
    #[test]
    #[should_panic]
    fn connectivity_test() {
        let sym = ClashUtil::build_test();
        println!("{:?}", sym.check_connectivity().unwrap_err());
    }
}
