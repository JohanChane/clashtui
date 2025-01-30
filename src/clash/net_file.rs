use super::*;

pub fn get_blob<U: Into<minreq::URL>, S: Into<String>>(
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
    req.with_timeout(*TIMEOUT).send_lazy().map_err(|e| e.into())
}
