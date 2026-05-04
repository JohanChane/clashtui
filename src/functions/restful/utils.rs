use super::*;

macro_rules! timeout {
    () => {
        CONFIG.cfg_file.timeout.unwrap_or(DEFAULT_TIMEOUT)
    };
}

pub fn request(
    method: minreq::Method,
    sub_url: &str,
    payload: Option<String>,
) -> Result<minreq::Response> {
    let mut req = minreq::Request::new(method, CONFIG.external_controller.clone() + sub_url);
    if let Some(kv) = payload {
        req = req
            .with_header("Content-Type", "application/json")
            .with_body(kv);
    }
    if let Some(s) = CONFIG.secret.as_ref() {
        req = req.with_header(headers::AUTHORIZATION, format!("Bearer {s}"));
    }
    req.with_timeout(timeout!()).send().map_err(|e| e.into())
}
