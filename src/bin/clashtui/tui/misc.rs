#[derive(PartialEq, Eq)]
pub enum EventState {
    Yes,
    Cancel,
    NotConsumed,
    WorkDone,
}
#[allow(unused)]
impl EventState {
    /// Returns true if **NOT** eq to [`EventState::NotConsumed`].
    pub fn is_consumed(&self) -> bool {
        !self.is_notconsumed()
    }
    /// Returns true if eq to [`EventState::NotConsumed`].
    pub fn is_notconsumed(&self) -> bool {
        self == &Self::NotConsumed
    }
    /// consume `Self`, return [`EventState::NotConsumed`] if [`EventState::is_notconsumed`],
    /// else return [`EventState::WorkDone`].
    pub fn unify(self) -> Self {
        if self.is_notconsumed() {
            Self::NotConsumed
        } else {
            Self::WorkDone
        }
    }
}
