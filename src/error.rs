use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    IO(#[from] std::io::Error),
    Net(#[from] minreq::Error),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{self:?}")
    }
}
