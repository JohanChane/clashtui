mod list;
mod list_popup;
pub mod tools;

pub use list::List;
pub use list_popup::ListPopup;

pub enum PopMsg {
    /// the first stand for the `question`,
    /// the sencond and third stand for the `extra choices`
    ///
    /// **NOTE**: support 2 extra choice only
    ///
    /// this will be like
    /// ```
    ///     `the question`
    /// Press y for Yes, n for No, o for `ch2`, t for `ch3`
    /// ```
    AskChoices(Vec<String>, Vec<String>),
    /// show infos
    Prompt(Vec<String>),
    SelectList(String, Vec<String>),
    Input(Vec<String>),
}

pub enum PopRes {
    Selected(usize),
    Selected_(usize),
    Input(Vec<String>),
}
