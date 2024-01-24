#[derive(Eq, Hash, PartialEq, Debug)]
pub enum Flags {
    UpdateOnly,
    FirstInit,
    ErrorDuringInit,
}
