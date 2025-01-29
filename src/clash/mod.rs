pub mod net_file;
pub mod util;
pub mod webapi;

mod error;
type CResult<T> = Result<T, error::Error>;

const DEFAULT_PAYLOAD: &str = r#"'{"path": "", "payload": ""}'"#;
const DEFAULT_TIMEOUT: u64 = 5;
static TIMEOUT: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
pub mod headers {
    pub const USER_AGENT: &str = "user-agent";
    pub const AUTHORIZATION: &str = "authorization";
    // TODO: change this
    pub const DEFAULT_USER_AGENT: &str = "github.com/celeo/github_version_check";
}
