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
        req = req.with_body(kv);
    }
    if let Some(s) = CONFIG.secret.as_ref() {
        req = req.with_header(headers::AUTHORIZATION, format!("Bearer {s}"));
    }
    req.with_timeout(timeout!()).send().map_err(|e| e.into())
}

#[cfg(feature = "deprecated")]
#[deprecated = "EC has been deprecated. User should run `echo -n \"user:password\" | base64` to get basic auth token"]
/// Convert Embedded Credential into Authorization Token
pub fn parse_request_with_cred(url: &str) -> Result<minreq::Request, minreq::Error> {
    let mut parsed_url = url_parse::core::Parser::new(None)
        .parse(url)
        .map_err(|_| minreq::Error::Other("Failed to parse URL"))?;

    if let (Some(username), Some(password)) = &parsed_url.user_pass {
        use base64::Engine as _;

        let auth_value = format!("{}:{}", username, password);
        let auth_header = format!(
            "Basic {}",
            base64::prelude::BASE64_STANDARD.encode(auth_value)
        );

        parsed_url.user_pass = (None, None);
        Ok(minreq::get(parsed_url.serialize()).with_header(headers::AUTHORIZATION, auth_header))
    } else {
        Ok(minreq::get(url))
    }
}
