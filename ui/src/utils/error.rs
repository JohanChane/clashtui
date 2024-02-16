#[derive(Debug)]
pub struct Infallable;

impl std::fmt::Display for Infallable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "End of Stream")
    }
}

impl std::error::Error for Infallable {}

impl From<Infallable> for std::io::Error {
    fn from(_: Infallable) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, "Should Not fail")
    }
}
