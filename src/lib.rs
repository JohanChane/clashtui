#![warn(clippy::all)]
#![deny(unsafe_code)]
pub mod backend;
mod error;
pub mod profile;
pub mod webapi;

pub type Result<T> = core::result::Result<T, Error>;
pub use error::Error;
