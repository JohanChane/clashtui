use std::{
    fmt::{Display, Write},
    str::FromStr,
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Key {
    pub code: KeyCode,
    #[serde(default, skip_serializing_if = "is_false")]
    pub shift: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub ctrl: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub alt: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub super_: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl Key {
    pub fn plain(&self) -> Option<char> {
        match self.code {
            KeyCode::Char(c) if !self.ctrl && !self.alt && !self.super_ => Some(c),
            _ => None,
        }
    }
}

impl Default for Key {
    fn default() -> Self {
        Self {
            code: KeyCode::Null,
            shift: false,
            ctrl: false,
            alt: false,
            super_: false,
        }
    }
}

impl From<KeyEvent> for Key {
    fn from(value: KeyEvent) -> Self {
        let shift = match (value.code, value.modifiers) {
            (KeyCode::Char(c), m) => {
                if c.is_ascii_uppercase() {
                    true
                } else if !c.is_ascii_alphabetic() && m.contains(KeyModifiers::SHIFT) {
                    false
                } else {
                    false
                }
            }
            (KeyCode::BackTab, _) => false,
            (_, m) => m.contains(KeyModifiers::SHIFT),
        };

        Self {
            code: value.code,
            shift,
            ctrl: value.modifiers.contains(KeyModifiers::CONTROL),
            alt: value.modifiers.contains(KeyModifiers::ALT),
            super_: value.modifiers.contains(KeyModifiers::SUPER),
        }
    }
}

impl FromStr for Key {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use anyhow::bail;

        if s.is_empty() {
            bail!("empty key");
        }

        let mut key = Self::default();
        if !s.starts_with('<') || !s.ends_with('>') {
            key.code = KeyCode::Char(s.chars().next().unwrap());
            key.shift = matches!(key.code, KeyCode::Char(c) if c.is_ascii_uppercase());
            return Ok(key);
        }

        let mut it = s[1..s.len() - 1].split_inclusive('-').peekable();
        while let Some(next) = it.next() {
            match next.to_ascii_lowercase().as_str() {
                "s-" => key.shift = true,
                "c-" => key.ctrl = true,
                "a-" => key.alt = true,
                "d-" => key.super_ = true,

                "space" => key.code = KeyCode::Char(' '),
                "backspace" => key.code = KeyCode::Backspace,
                "enter" => key.code = KeyCode::Enter,
                "left" => key.code = KeyCode::Left,
                "right" => key.code = KeyCode::Right,
                "up" => key.code = KeyCode::Up,
                "down" => key.code = KeyCode::Down,
                "home" => key.code = KeyCode::Home,
                "end" => key.code = KeyCode::End,
                "pageup" => key.code = KeyCode::PageUp,
                "pagedown" => key.code = KeyCode::PageDown,
                "tab" => key.code = KeyCode::Tab,
                "backtab" => key.code = KeyCode::BackTab,
                "delete" => key.code = KeyCode::Delete,
                "insert" => key.code = KeyCode::Insert,
                "esc" => key.code = KeyCode::Esc,

                _ => match next {
                    s if it.peek().is_none() => {
                        let c = s.chars().next().unwrap();
                        key.shift |= c.is_ascii_uppercase();
                        key.code =
                            KeyCode::Char(if key.shift { c.to_ascii_uppercase() } else { c });
                    }
                    s => bail!("unknown key: {s}"),
                },
            }
        }

        if key.code == KeyCode::Null {
            bail!("empty key");
        }
        Ok(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn plain_char_no_modifiers() {
        let k = Key::from(key_event(KeyCode::Char('a'), KeyModifiers::NONE));
        assert_eq!(k.code, KeyCode::Char('a'));
        assert!(!k.shift);
        assert!(!k.ctrl);
        assert!(!k.alt);
        assert!(!k.super_);
    }

    #[test]
    fn uppercase_char_infers_shift() {
        let k = Key::from(key_event(KeyCode::Char('A'), KeyModifiers::NONE));
        assert_eq!(k.code, KeyCode::Char('A'));
        assert!(k.shift);
    }

    #[test]
    fn uppercase_char_with_shift_modifier() {
        let k = Key::from(key_event(KeyCode::Char('A'), KeyModifiers::SHIFT));
        assert_eq!(k.code, KeyCode::Char('A'));
        assert!(k.shift);
    }

    #[test]
    fn ctrl_key() {
        let k = Key::from(key_event(KeyCode::Char('w'), KeyModifiers::CONTROL));
        assert_eq!(k.code, KeyCode::Char('w'));
        assert!(k.ctrl);
        assert!(!k.shift);
    }

    #[test]
    fn non_alpha_shift_stripped_windows_style() {
        let k = Key::from(key_event(KeyCode::Char('~'), KeyModifiers::SHIFT));
        assert_eq!(k.code, KeyCode::Char('~'));
        assert!(!k.shift, "shift should be stripped for non-alpha char");
    }

    #[test]
    fn non_alpha_shift_stripped_unix_style() {
        let k = Key::from(key_event(KeyCode::Char('~'), KeyModifiers::NONE));
        assert_eq!(k.code, KeyCode::Char('~'));
        assert!(!k.shift);
    }

    #[test]
    fn shift_digit_normalized() {
        let k_win = Key::from(key_event(KeyCode::Char('!'), KeyModifiers::SHIFT));
        let k_unix = Key::from(key_event(KeyCode::Char('!'), KeyModifiers::NONE));
        assert_eq!(k_win, k_unix);
        assert_eq!(k_win.code, KeyCode::Char('!'));
        assert!(!k_win.shift);
    }

    #[test]
    fn lowercase_alpha_stays_unshifted() {
        let k = Key::from(key_event(KeyCode::Char('a'), KeyModifiers::NONE));
        assert_eq!(k.code, KeyCode::Char('a'));
        assert!(!k.shift);
    }

    #[test]
    fn backtab_shift_is_false() {
        let k = Key::from(key_event(KeyCode::BackTab, KeyModifiers::SHIFT));
        assert_eq!(k.code, KeyCode::BackTab);
        assert!(!k.shift);
    }

    #[test]
    fn non_char_key_with_shift() {
        let k = Key::from(key_event(KeyCode::Up, KeyModifiers::SHIFT));
        assert_eq!(k.code, KeyCode::Up);
        assert!(k.shift);
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(c) = self.plain() {
            return if c == ' ' {
                write!(f, "<Space>")
            } else {
                f.write_char(c)
            };
        }

        write!(f, "<")?;
        if self.super_ {
            write!(f, "D-")?;
        }
        if self.ctrl {
            write!(f, "C-")?;
        }
        if self.alt {
            write!(f, "A-")?;
        }
        if self.shift && !matches!(self.code, KeyCode::Char(_)) {
            write!(f, "S-")?;
        }

        let code = match self.code {
            KeyCode::Backspace => "Backspace",
            KeyCode::Enter => "Enter",
            KeyCode::Left => "Left",
            KeyCode::Right => "Right",
            KeyCode::Up => "Up",
            KeyCode::Down => "Down",
            KeyCode::Home => "Home",
            KeyCode::End => "End",
            KeyCode::PageUp => "PageUp",
            KeyCode::PageDown => "PageDown",
            KeyCode::Tab => "Tab",
            KeyCode::BackTab => "BackTab",
            KeyCode::Delete => "Delete",
            KeyCode::Insert => "Insert",
            KeyCode::Esc => "Esc",

            KeyCode::Char(' ') => "Space",
            KeyCode::Char(c) => {
                f.write_char(c)?;
                ""
            }
            _ => "Unknown",
        };

        write!(f, "{code}>")
    }
}
