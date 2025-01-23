use super::*;
pub mod self_update;

/// try GET raw data from given `url`
///
/// return an object that impl [Read](std::io::Read)
pub fn get_file(url: &str) -> CResult<minreq::ResponseLazy> {
    use super::headers;
    get_blob(url, None, Some(headers::DEFAULT_USER_AGENT))
}

pub fn get_blob<U: Into<minreq::URL>, S: Into<String>>(
    // &self,
    url: U,
    proxy: Option<&str>,
    ua: Option<S>,
) -> CResult<minreq::ResponseLazy> {
    let mut req = minreq::get(url);
    if let Some(proxy) = proxy {
        req = req.with_proxy(minreq::Proxy::new(proxy)?)
    }
    if let Some(ua) = ua {
        req = req.with_header(headers::USER_AGENT, ua)
    }
    req.with_timeout(*TIMEOUT.get().unwrap())
        .send_lazy()
        .map_err(|e| e.into())
}
