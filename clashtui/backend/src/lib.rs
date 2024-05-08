#![warn(clippy::all)]
pub mod utils;
pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"));
/// re-export
pub mod api {
    pub use api::Mode;
}
