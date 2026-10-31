use super::runtime::{DispatchEntry, KeySequence};
use super::{Action, ChordView, Key, KeyCode, KeyModifiers, Keymaps, Scope};

#[derive(Clone, Copy)]
pub(super) enum InputPolicy {
    Exact,
    WindowsNative,
}

impl InputPolicy {
    fn character_fallback(self, key: Key) -> Option<Key> {
        // Crossterm's native Windows reader reports the generated character and
        // Shift together. VT/Kitty events can have different character semantics.
        if matches!(self, Self::WindowsNative)
            && key.modifiers == KeyModifiers::SHIFT
            && let KeyCode::Char(c) = key.code
            && !c.is_whitespace()
            && !c.is_control()
        {
            Some(Key {
                code: key.code,
                modifiers: KeyModifiers::NONE,
            })
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Match<A> {
    Pending,
    Execute(A),
    Cancelled,
    Unmatched,
}

struct Pending {
    // Focus at the first key; a focus change cancels this sequence.
    input_scope: Scope,
    // Keep the original candidate set, including chords started from help.
    map_scope: Scope,
    prefix: Key,
}

/// One input state owned by App; maps and their presentation data stay immutable.
pub struct Matcher<'a> {
    keymaps: &'a Keymaps,
    pending: Option<Pending>,
    policy: InputPolicy,
}

impl<'a> Matcher<'a> {
    pub fn new(keymaps: &'a Keymaps) -> Self {
        // The current Windows backend uses crossterm's native console reader.
        // Revisit this policy if that backend switches to VT/Kitty input.
        let policy = if cfg!(windows) {
            InputPolicy::WindowsNative
        } else {
            InputPolicy::Exact
        };
        Self::with_policy(keymaps, policy)
    }

    pub(super) fn with_policy(keymaps: &'a Keymaps, policy: InputPolicy) -> Self {
        Self {
            keymaps,
            pending: None,
            policy,
        }
    }

    fn first(&self, scope: Scope, key: Key) -> Option<(Key, &'a DispatchEntry)> {
        if let Some(entry) = self.keymaps.lookup(scope, &key) {
            return Some((key, entry));
        }
        if self.keymaps.has_disabled_prefix(scope, key) {
            return None;
        }
        let fallback = self.policy.character_fallback(key)?;
        self.keymaps
            .lookup(scope, &fallback)
            .map(|entry| (fallback, entry))
    }

    pub fn starts_chord(&self, scope: Scope, key: &Key) -> bool {
        matches!(self.first(scope, *key), Some((_, DispatchEntry::Chord(_))))
    }

    pub fn feed(&mut self, key: &Key, input_scope: Scope, map_scope: Scope) -> Match<Action> {
        if let Some(pending) = self.pending.take()
            && pending.input_scope == input_scope
        {
            if *key == Key::from_code(KeyCode::Esc) {
                return Match::Cancelled;
            }
            return self
                .keymaps
                .second(pending.map_scope, &pending.prefix, key)
                .or_else(|| {
                    if self.keymaps.is_disabled(
                        pending.map_scope,
                        KeySequence::Chord([pending.prefix, *key]),
                    ) {
                        return None;
                    }
                    let fallback = self.policy.character_fallback(*key)?;
                    self.keymaps
                        .second(pending.map_scope, &pending.prefix, &fallback)
                })
                .map(|id| Match::Execute(self.keymaps.binding(id).action))
                .unwrap_or(Match::Cancelled);
        }
        match self.first(map_scope, *key) {
            Some((_, DispatchEntry::Single(id))) => {
                Match::Execute(self.keymaps.binding(*id).action)
            }
            Some((prefix, DispatchEntry::Chord(_))) => {
                self.pending = Some(Pending {
                    input_scope,
                    map_scope,
                    prefix,
                });
                Match::Pending
            }
            None => Match::Unmatched,
        }
    }

    pub fn hints(&self, input_scope: Scope) -> Option<ChordView<'a>> {
        self.pending
            .as_ref()
            .filter(|pending| pending.input_scope == input_scope)
            .and_then(|pending| self.keymaps.chord_view(pending.map_scope, pending.prefix))
    }

    pub fn cancel(&mut self) {
        self.pending = None;
    }
}
