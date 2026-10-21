use super::{Key, KeyCode, KeyMap, MaybeMap, Scope, SubMap, WhichHints};

#[derive(Debug, PartialEq, Eq)]
pub enum Match<A> {
    Pending,
    Execute(A),
    Cancelled,
    Unmatched,
}

struct Pending<'a, A> {
    scope: Scope,
    submap: &'a SubMap<A>,
}

/// One input state owned by App; maps and their presentation data stay immutable.
pub struct Matcher<'a, A> {
    pending: Option<Pending<'a, A>>,
}

impl<A> Default for Matcher<'_, A> {
    fn default() -> Self {
        Self { pending: None }
    }
}

impl<'a, A: Copy> Matcher<'a, A> {
    pub fn feed(&mut self, key: &Key, scope: Scope, map: &'a KeyMap<A>) -> Match<A> {
        if let Some(pending) = self.pending.take()
            && pending.scope == scope
        {
            if *key == Key::from_code(KeyCode::Esc) {
                return Match::Cancelled;
            }
            return pending
                .submap
                .inner
                .get(key)
                .copied()
                .map(Match::Execute)
                .unwrap_or(Match::Cancelled);
        }
        match map.get(key) {
            Some(MaybeMap::Action(action)) => Match::Execute(*action),
            Some(MaybeMap::SubMap(submap)) => {
                self.pending = Some(Pending { scope, submap });
                Match::Pending
            }
            None => Match::Unmatched,
        }
    }

    pub fn hints(&self, scope: Scope) -> Option<&'a WhichHints> {
        self.pending
            .as_ref()
            .filter(|pending| pending.scope == scope)
            .map(|pending| &pending.submap.hints)
    }

    pub fn cancel(&mut self) {
        self.pending = None;
    }
}
