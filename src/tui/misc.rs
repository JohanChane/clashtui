#[derive(PartialEq, Eq, Debug, Default, Clone, Copy)]
pub enum EventState {
    Yes,
    Cancel,
    #[default]
    NotConsumed,
    WorkDone,
}

#[allow(unused)]
impl EventState {
    /// Returns true if **NOT** equal to [`EventState::NotConsumed`].
    pub fn is_consumed(&self) -> bool {
        !self.is_notconsumed()
    }

    /// Returns true if equal to [`EventState::NotConsumed`].
    pub fn is_notconsumed(&self) -> bool {
        self == &Self::NotConsumed
    }

    /// Consumes `self` and returns [`EventState::NotConsumed`] if [`EventState::is_notconsumed`],
    /// else returns [`EventState::WorkDone`].
    pub fn unify(self) -> Self {
        if self.is_notconsumed() {
            Self::NotConsumed
        } else {
            Self::WorkDone
        }
    }
}
