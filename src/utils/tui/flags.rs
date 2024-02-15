#[derive(Eq, Hash, PartialEq, Debug)]
pub enum Flag {
    UpdateOnly,
    FirstInit,
    ErrorDuringInit,
    PortableMode,
}
#[derive(Debug)]
pub struct Flags {
    inner: std::collections::HashSet<Flag>,
}
impl Flags {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: std::collections::HashSet::with_capacity(capacity),
        }
    }
    pub fn insert(&mut self, k: Flag) {
        // current, flag should not be add more than once
        if !self.inner.insert(k) {
            panic!();
        }
    }
    pub fn contains(&self, k: Flag) -> bool {
        self.inner.contains(&k)
    }
}
