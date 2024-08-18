#![warn(clippy::all)]
mod backend;
mod consts;
pub mod utils;
pub use backend::ClashBackend;
pub use consts::VERSION;
/// re-export
pub mod api {
    pub use crate::api::Mode;
}
pub use consts::const_err;
