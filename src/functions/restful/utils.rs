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
    let controller = CONFIG.controller_for_core();
    let mut req = minreq::Request::new(method, format!("{controller}{sub_url}"));
    if let Some(kv) = payload {
        req = req
            .with_header("Content-Type", "application/json")
            .with_body(kv);
    }
    if let Some(s) = CONFIG.secret_for_core() {
        req = req.with_header(headers::AUTHORIZATION, format!("Bearer {s}"));
    }
    req.with_timeout(timeout!()).send().map_err(|e| e.into())
}
