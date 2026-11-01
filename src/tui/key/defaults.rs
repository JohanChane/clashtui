use super::{Key, KeySequence};

/// Built-in bindings in declaration order; no configuration serialization needed.
pub(in crate::tui) struct DefaultBindings<A> {
    pub(super) bindings: Vec<(KeySequence, A)>,
}

impl<A> DefaultBindings<A> {
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    pub fn with_keys(mut self, bindings: impl IntoIterator<Item = (impl Into<Key>, A)>) -> Self {
        self.bindings.extend(
            bindings
                .into_iter()
                .map(|(key, action)| (KeySequence::Single(key.into().normalize()), action)),
        );
        self
    }

    pub fn with_chords(
        mut self,
        prefix: impl Into<Key>,
        bindings: impl IntoIterator<Item = (impl Into<Key>, A)>,
    ) -> Self {
        let prefix = prefix.into().normalize();
        self.bindings.extend(
            bindings.into_iter().map(|(key, action)| {
                (KeySequence::Chord([prefix, key.into().normalize()]), action)
            }),
        );
        self
    }
}
