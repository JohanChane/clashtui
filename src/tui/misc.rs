#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum EventState {
    /// a work is done and we should do something
    Yes,
    /// a work is canceled and we might do something
    Cancel,
    /// this widget failed to process this key,
    /// have the next widget do it
    NotConsumed,
    /// this widget processed this key
    /// and we are all done here
    Consumed,
}

impl EventState {
    /// Consumes `self` and returns [`EventState::NotConsumed`] if [`EventState::is_notconsumed`],
    /// else returns [`EventState::WorkDone`].
    pub fn unify(self) -> Self {
        if matches!(self, Self::NotConsumed) {
            Self::NotConsumed
        } else {
            Self::Consumed
        }
    }
}
