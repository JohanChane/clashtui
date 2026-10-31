use std::fmt::Display;
use std::io::Write;

#[allow(dead_code)]
pub struct Select<It> {
    start_prompts: Vec<String>,
    end_prompt: Option<String>,
    items: Vec<It>,
}
impl<It: Display> Default for Select<It> {
    fn default() -> Self {
        Self {
            start_prompts: Default::default(),
            end_prompt: None,
            items: Default::default(),
        }
    }
}
/// A utility struct for interacting with a list of items.
///
/// The `Select` struct provides methods for appending items to the list, setting prompts, and
/// interacting with the user to select an item from the list.
#[allow(dead_code)]
impl<It: Display> Select<It> {
    pub fn append_items<I: Iterator<Item = It>>(mut self, items: I) -> Self {
        self.items.extend(items);
        self
    }
    pub fn append_start_prompt<S: ToString>(mut self, prompt: S) -> Self {
        self.start_prompts.push(prompt.to_string());
        self
    }

    /// Sets the end prompt.
    ///
    /// ### Panics
    ///
    /// Panics if `set_end_prompt` is called twice.
    pub fn set_end_prompt<S: Display>(mut self, prompt: S) -> Self {
        assert_eq!(
            self.end_prompt.replace(format!("{prompt} [0..9/n]: ")),
            None,
            "set_end_prompt is called twice"
        );
        self
    }

    /// Shows a list of items with index numbers and interacts with the user to select an item.
    ///
    /// Users can type the number of the item to select it, or 'n'/'N' to return `None`.
    ///
    /// ### Returns
    ///
    /// A [std::io::Result] containing [Some]\(item) if an item is selected, or [None] if 'n'/'N' is selected.
    pub fn interact(self) -> std::io::Result<Option<It>> {
        let Self {
            start_prompts,
            end_prompt,
            mut items,
        } = self;
        debug_assert!(!items.is_empty(), "Empty list for Select, why it's here?");
        let mut out = std::io::stderr().lock();
        loop {
            start_prompts
                .iter()
                .try_for_each(|prompt| writeln!(out, "{}", prompt))?;
            writeln!(out)?;
            items
                .iter()
                .enumerate()
                .try_for_each(|(idx, item)| writeln!(out, "{:0>2} {}", idx, item))?;
            writeln!(out)?;
            if let Some(end_prompt) = &end_prompt {
                write!(out, "{}", end_prompt)?;
            };

            let mut buf = String::new();
            std::io::stdin().read_line(&mut buf)?;
            match buf.trim().parse() {
                Ok(cur) => {
                    if items.len() > cur {
                        return Ok(Some(items.remove(cur)));
                    }
                }
                Err(e) => {
                    if matches!(buf.chars().nth(0), Some('n') | Some('N')) {
                        return Ok(None);
                    }
                    eprintln!("Not a valid input: {e}");
                }
            };
        }
    }
}
