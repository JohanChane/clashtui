mod list_popup;
mod list;
mod popup;
pub mod tools;

pub use list::List;
pub use popup::ConfirmPopup;
pub use list_popup::ListPopup;

pub enum PopMsg {
    /// the first stand for the `question`,
    /// the sencond and third stand for the `extra choices`
    ///
    /// this will be like
    /// ```md
    ///     `the question`
    /// Press y for Yes, n for No, o for `ch2`, t for `ch3`
    /// ```
    Ask(Vec<String>, Option<String>, Option<String>),
    /// show infos
    Prompt(Vec<String>),
}
