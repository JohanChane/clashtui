use super::runtime::DispatchEntry;
use super::{Action, ChordView, Key, KeyCode, Keymaps, Scope};

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
}

impl<'a> Matcher<'a> {
    pub fn new(keymaps: &'a Keymaps) -> Self {
        Self {
            keymaps,
            pending: None,
        }
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
                .map(|id| Match::Execute(self.keymaps.binding(id).action))
                .unwrap_or(Match::Cancelled);
        }
        match self.keymaps.lookup(map_scope, key) {
            Some(DispatchEntry::Single(id)) => Match::Execute(self.keymaps.binding(*id).action),
            Some(DispatchEntry::Chord(_)) => {
                self.pending = Some(Pending {
                    input_scope,
                    map_scope,
                    prefix: *key,
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
