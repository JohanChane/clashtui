#![warn(clippy::all)]
mod backend;
mod consts;
pub mod utils;
pub use backend::ClashBackend;
pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"));
/// re-export
pub mod api {
    pub use api::Mode;
}
pub use consts::const_err;
