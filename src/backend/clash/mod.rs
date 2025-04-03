pub mod webapi;

mod error;
type CResult<T> = Result<T, error::Error>;
pub type MinreqResult = Result<minreq::ResponseLazy, minreq::Error>;

const DEFAULT_PAYLOAD: &str = r#"'{"path": "", "payload": ""}'"#;
const DEFAULT_TIMEOUT: u64 = 5;

pub mod headers {
    pub const USER_AGENT: &str = "user-agent";
    pub const AUTHORIZATION: &str = "authorization";
    pub const DEFAULT_USER_AGENT: &str = "github.com/JohanChane/clashtui";
}

pub fn get_blob<U: Into<minreq::URL>>(
    url: U,
    proxy: Option<&str>,
    ua: Option<&str>,
    timeout: u64,
) -> MinreqResult {
    let mut req = make_request_with_cred(url)?;
    if let Some(proxy) = proxy {
        req = req.with_proxy(minreq::Proxy::new(proxy)?)
    }
    if let Some(ua) = ua {
        req = req.with_header(headers::USER_AGENT, ua)
    }
    req.with_timeout(timeout).send_lazy()
}

// Support URL with Embedded Credentials
pub fn make_request_with_cred<U: Into<minreq::URL>>(
    url: U,
) -> Result<minreq::Request, minreq::Error> {
    use base64::Engine;
    use url::Url;

    let url_str = url.into().to_string();

    let parsed_url =
        Url::parse(&url_str).map_err(|_| minreq::Error::Other("Failed to parse URL"))?;

    let username = parsed_url.username();
    let password = parsed_url.password().unwrap_or("");

    let mut request_url = parsed_url.clone();
    request_url
        .set_username("")
        .map_err(|_| minreq::Error::Other("Failed to clear username"))?;
    request_url
        .set_password(None)
        .map_err(|_| minreq::Error::Other("Failed to clear password"))?;

    let auth_value = format!("{}:{}", username, password);
    let auth_header = format!(
        "Basic {}",
        base64::prelude::BASE64_STANDARD.encode(auth_value)
    );

    Ok(minreq::get(request_url.as_str()).with_header("Authorization", auth_header))
}
