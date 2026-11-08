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
    if sub_url != "/version" && crate::config::is_core_mismatch() {
        return Err(minreq::Error::IoError(std::io::Error::other(
            "core mismatch",
        )));
    }
    let controller = CONFIG.controller_for_core();
    request_to(
        method,
        controller,
        CONFIG.secret_for_core(),
        sub_url,
        payload,
        timeout!(),
    )
}

/// Explicit target for operations that must survive a change of selected core.
pub fn request_to(
    method: minreq::Method,
    controller: &str,
    secret: Option<&str>,
    sub_url: &str,
    payload: Option<String>,
    timeout_secs: u64,
) -> Result<minreq::Response> {
    let mut req = minreq::Request::new(method, format!("{controller}{sub_url}"));
    if let Some(kv) = payload {
        req = req
            .with_header("Content-Type", "application/json")
            .with_body(kv);
    }
    if let Some(s) = secret {
        req = req.with_header(headers::AUTHORIZATION, format!("Bearer {s}"));
    }
    req.with_timeout(timeout_secs).send()
}
