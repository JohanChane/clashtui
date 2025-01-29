use std::fmt::Display;
use std::io::Write;

#[derive(Default)]
pub struct Confirm {
    prompts: Vec<String>,
}
impl Confirm {
    pub fn append_prompt<S: ToString>(mut self, prompt: S) -> Self {
        self.prompts.push(prompt.to_string());
        self
    }
    pub fn interact(self) -> std::io::Result<bool> {
        let Self { mut prompts } = self;
        debug_assert!(
            !prompts.is_empty(),
            "Empty prompt for Confirm, why it's here?"
        );
        let mut out = std::io::stderr().lock();
        let prompt = prompts.pop().unwrap();
        for p in prompts {
            writeln!(out, "{}", p)?;
        }
        loop {
            write!(out, "{prompt} [y/n] ")?;
            let mut buf = String::new();
            std::io::stdin().read_line(&mut buf)?;
            return match buf.chars().nth(0) {
                Some('y') | Some('Y') => Ok(true),
                Some('n') | Some('N') => Ok(false),
                _ => {
                    eprintln!("Not a valid input");
                    continue;
                }
            };
        }
    }
}

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
impl<It: Display> Select<It> {
    pub fn append_items<I: Iterator<Item = It>>(mut self, items: I) -> Self {
        self.items.extend(items);
        self
    }
    pub fn append_start_prompt<S: ToString>(mut self, prompt: S) -> Self {
        self.start_prompts.push(prompt.to_string());
        self
    }
    pub fn set_end_prompt<S: ToString>(mut self, prompt: S) -> Self {
        assert_eq!(
            self.end_prompt.replace(prompt.to_string()),
            None,
            "set_end_prompt is called twice"
        );
        self
    }
    pub fn interact(self) -> std::io::Result<It> {
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
            items
                .iter()
                .enumerate()
                .try_for_each(|(i, item)| writeln!(out, "{i} {}", item))?;
            if let Some(end_prompt) = &end_prompt {
                write!(out, "{}", end_prompt)?;
            };

            let mut buf = String::new();
            std::io::stdin().read_line(&mut buf)?;
            match buf.trim().parse() {
                Ok(cur) => {
                    if items.len() > cur {
                        return Ok(items.remove(cur));
                    }
                }
                Err(e) => {
                    eprintln!("Not a valid input: {e}");
                }
            };
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    #[ignore = "Manual only"]
    fn select() -> std::io::Result<()> {
        #[derive(Debug)]
        struct D1 {
            name: u8,
        }
        impl std::fmt::Display for D1 {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.name)
            }
        }
        let that = Select::default()
            .append_start_prompt("p1:")
            .append_start_prompt("p2")
            .set_end_prompt("p3:")
            .append_items([D1 { name: 1 }, D1 { name: 3 }].into_iter())
            .interact()?;
        println!("Select {:?}", that);
        Ok(())
    }
}
