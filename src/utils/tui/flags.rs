#[derive(Eq, Hash, PartialEq, Debug)]
pub enum Flag {
    UpdateOnly,
    FirstInit,
    ErrorDuringInit,
    PortableMode,
}
#[derive(Debug)]
pub struct Flags {
    inner: std::collections::HashMap<Flag, ()>,
}
impl Flags {
    pub fn new() -> Self {
        Self {
            inner: std::collections::HashMap::new(),
        }
    }
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: std::collections::HashMap::with_capacity(capacity),
        }
    }
    pub fn insert(&mut self, k: Flag) {
        // current, flag should not be add more than once
        if let Some(_) = self.inner.insert(k, ()) {
            panic!();
        }
    }
    pub fn contains_key(&self, k: Flag) -> bool {
        self.inner.contains_key(&k)
    }
}