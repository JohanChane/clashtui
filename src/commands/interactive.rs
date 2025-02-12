use std::fmt::Display;

#[derive(Default)]
pub struct Confirm {
    prompts: Vec<String>,
}
impl Confirm {
    pub fn append_prompt<S: ToString>(mut self, prompt: S) -> Self {
        self.prompts.push(prompt.to_string());
        self
    }
    pub fn interact(mut self) -> std::io::Result<bool> {
        assert_ne!(self.prompts.len(), 0);
        use std::io::{Read, Write};
        let mut out = std::io::stderr().lock();
        let prompt = self.prompts.pop().unwrap();
        for p in self.prompts {
            writeln!(out, "{}", p)?;
        }
        write!(out, "{prompt} [y/n] ")?;
        let mut buf = [0];
        std::io::stdin().read_exact(&mut buf)?;
        match buf[0] {
            b'y' | b'Y' => Ok(true),
            b'n' | b'N' => Ok(false),
            _ => Err(std::io::Error::other("Not a valid input")),
        }
    }
}

pub struct Select<It> {
    start_prompts: Vec<String>,
    end_prompts: Vec<String>,
    items: Vec<It>,
}
impl<It: Display> Default for Select<It> {
    fn default() -> Self {
        Self {
            start_prompts: Default::default(),
            end_prompts: Default::default(),
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
    pub fn append_end_prompt<S: ToString>(mut self, prompt: S) -> Self {
        self.end_prompts.push(prompt.to_string());
        self
    }
    pub fn interact(self) -> std::io::Result<It> {
        use std::io::{Read, Write};
        let Self {
            start_prompts,
            mut end_prompts,
            mut items,
        } = self;
        if items.is_empty() {
            return Err(std::io::Error::other("No item"));
        }
        let mut out = std::io::stderr().lock();
        let end_prompt = end_prompts.pop();
        loop {
            start_prompts
                .iter()
                .try_for_each(|prompt| writeln!(out, "{}", prompt))?;
            items
                .iter()
                .enumerate()
                .try_for_each(|(i, item)| writeln!(out, "{i} {}", item))?;
            if let Some(end_prompt) = &end_prompt {
                end_prompts
                    .iter()
                    .try_for_each(|prompt| writeln!(out, "{}", prompt))?;
                write!(out, "{}", end_prompt)?;
            };

            let mut buf = [0, 0];
            std::io::stdin().read_exact(&mut buf)?;
            let mut cur: usize = 0;
            for byte in buf {
                if byte == 10 {
                    break;
                }
                assert!(
                    byte > 47 && byte < 58,
                    "This({}) is not a valid ascii number",
                    byte
                );
                let num = byte - 48;
                cur = cur * 10 + num as usize;
            }
            if items.len() > cur {
                return Ok(items.remove(cur));
            }
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
            .append_end_prompt("p3")
            .append_end_prompt("p4:")
            .append_items([D1 { name: 1 }, D1 { name: 3 }].into_iter())
            .interact()?;
        println!("Select {:?}", that);
        Ok(())
    }
}
