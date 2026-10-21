use super::{Key, KeyCode, KeyModifiers};
use std::{fmt, str::FromStr};

impl FromStr for Key {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut modifiers = KeyModifiers::empty();
        let code = if text.chars().count() == 1 {
            KeyCode::Char(text.chars().next().unwrap())
        } else {
            let mut name = text
                .strip_prefix('<')
                .and_then(|s| s.strip_suffix('>'))
                .ok_or_else(|| anyhow::anyhow!("invalid key `{text}`; use one character or <Key>; write chords as [\"g\", \"g\"]"))?;
            while let Some((prefix, rest)) = name.split_once('-') {
                let modifier = match prefix.to_ascii_lowercase().as_str() {
                    "c" => KeyModifiers::CONTROL,
                    "s" => KeyModifiers::SHIFT,
                    "a" => KeyModifiers::ALT,
                    "d" => KeyModifiers::SUPER,
                    _ => break,
                };
                modifiers.insert(modifier);
                name = rest;
            }
            match name.to_ascii_lowercase().as_str() {
                "space" => KeyCode::Char(' '),
                "backspace" => KeyCode::Backspace,
                "enter" => KeyCode::Enter,
                "left" => KeyCode::Left,
                "right" => KeyCode::Right,
                "up" => KeyCode::Up,
                "down" => KeyCode::Down,
                "home" => KeyCode::Home,
                "end" => KeyCode::End,
                "pageup" => KeyCode::PageUp,
                "pagedown" => KeyCode::PageDown,
                "tab" => KeyCode::Tab,
                "backtab" => KeyCode::BackTab,
                "delete" => KeyCode::Delete,
                "insert" => KeyCode::Insert,
                "esc" => KeyCode::Esc,
                _ if name.chars().count() == 1 => KeyCode::Char(name.chars().next().unwrap()),
                _ => {
                    let number = name.strip_prefix('F').or_else(|| name.strip_prefix('f'));
                    let number = number.and_then(|s| s.parse::<u8>().ok());
                    match number {
                        Some(n @ 1..=24) => KeyCode::F(n),
                        _ => anyhow::bail!("unknown key `{text}`"),
                    }
                }
            }
        };
        Ok(Self { code, modifiers }.normalize())
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut modifiers = self.modifiers;
        if matches!(self.code, KeyCode::Char(_)) {
            modifiers.remove(KeyModifiers::SHIFT);
        }
        if let KeyCode::Char(c) = self.code
            && modifiers.is_empty()
            && c != ' '
        {
            return write!(f, "{c}");
        }
        write!(f, "<")?;
        for (modifier, prefix) in [
            (KeyModifiers::SUPER, "D-"),
            (KeyModifiers::CONTROL, "C-"),
            (KeyModifiers::ALT, "A-"),
            (KeyModifiers::SHIFT, "S-"),
        ] {
            if modifiers.contains(modifier) {
                write!(f, "{prefix}")?;
            }
        }
        match self.code {
            KeyCode::Char(' ') => write!(f, "Space")?,
            KeyCode::Char(c) => write!(f, "{c}")?,
            KeyCode::Backspace => write!(f, "Backspace")?,
            KeyCode::Enter => write!(f, "Enter")?,
            KeyCode::Left => write!(f, "Left")?,
            KeyCode::Right => write!(f, "Right")?,
            KeyCode::Up => write!(f, "Up")?,
            KeyCode::Down => write!(f, "Down")?,
            KeyCode::Home => write!(f, "Home")?,
            KeyCode::End => write!(f, "End")?,
            KeyCode::PageUp => write!(f, "PageUp")?,
            KeyCode::PageDown => write!(f, "PageDown")?,
            KeyCode::Tab => write!(f, "Tab")?,
            KeyCode::BackTab => write!(f, "BackTab")?,
            KeyCode::Delete => write!(f, "Delete")?,
            KeyCode::Insert => write!(f, "Insert")?,
            KeyCode::Esc => write!(f, "Esc")?,
            KeyCode::F(n) => write!(f, "F{n}")?,
            other => write!(f, "{other}")?,
        }
        write!(f, ">")
    }
}

pub(super) fn format_sequence(keys: &[Key]) -> String {
    keys.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ")
}
