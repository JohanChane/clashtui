#[derive(Eq, Hash, PartialEq, Debug)]
pub enum Flag {
    UpdateOnly,
    FirstInit,
    ErrorDuringInit,
    PortableMode,
}
#[derive(Debug)]
pub struct Flags(std::collections::HashSet<Flag>);
impl Flags {
    pub fn with_capacity(capacity: usize) -> Self {
        Self(std::collections::HashSet::with_capacity(capacity))
    }
    pub fn insert(&mut self, k: Flag) {
        // current, flag should not be add more than once
        if !self.0.insert(k) {
            unreachable!()
        }
    }
    pub fn contains(&self, k: Flag) -> bool {
        self.0.contains(&k)
    }
}
