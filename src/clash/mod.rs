pub mod webapi;

mod error;
type CResult<T> = Result<T, error::Error>;
pub type MinreqResult = Result<minreq::ResponseLazy, minreq::Error>;

const DEFAULT_PAYLOAD: &str = r#"'{"path": "", "payload": ""}'"#;
const DEFAULT_TIMEOUT: u64 = 5;
static _TIMEOUT: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
static TIMEOUT: std::sync::LazyLock<u64> =
    std::sync::LazyLock::new(|| *_TIMEOUT.get().unwrap_or(&DEFAULT_TIMEOUT));

pub mod headers {
    pub const USER_AGENT: &str = "user-agent";
    pub const AUTHORIZATION: &str = "authorization";
    // TODO: change this
    pub const DEFAULT_USER_AGENT: &str = "github.com/celeo/github_version_check";
}

pub fn get_blob<U: Into<minreq::URL>>(
    url: U,
    proxy: Option<&str>,
    ua: Option<&str>,
) -> MinreqResult {
    let mut req = minreq::get(url);
    if let Some(proxy) = proxy {
        req = req.with_proxy(minreq::Proxy::new(proxy)?)
    }
    if let Some(ua) = ua {
        req = req.with_header(headers::USER_AGENT, ua)
    }
    req.with_timeout(*TIMEOUT).send_lazy()
}
