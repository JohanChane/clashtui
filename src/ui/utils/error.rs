/// This error means there should be `no` error
#[derive(Debug)]
pub struct Infailable;

impl std::fmt::Display for Infailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "End of Stream")
    }
}

impl std::error::Error for Infailable {}

impl From<Infailable> for std::io::Error {
    fn from(_: Infailable) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, "Should Not fail")
    }
}
