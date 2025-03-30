mod browser;
mod list;
mod popup;
pub mod tools;

pub use browser::{Browser, Path};
pub use list::List;
pub use popup::Popup;

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
    AskChoices(String, Vec<String>),
    /// show infos
    Prompt(String),
    SelectList(String, Vec<String>),
    SelectMulti(String, Vec<String>),
    Input(String),
}
#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum PopRes {
    /// result of [PopMsg::AskChoices]
    // Choices(usize),
    /// result of [PopMsg::SelectList]
    Selected(usize),
    SelectedMulti(Vec<usize>),
    /// result of [PopMsg::Input]
    Input(String),
}
