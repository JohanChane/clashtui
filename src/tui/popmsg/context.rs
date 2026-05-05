use crate::tui::Key;
use crossterm::event::KeyCode;

pub struct Context {
    pub widget: WidgetState,
    text: Option<TextState>,
    focus_on_text: bool,
}
impl Context {
    /// Should not be called in [Cell](super::Cell)
    pub fn handle_key_event(&mut self, kv: &Key) {
        if !self.focus_on_text {
            self.widget.handle_key_event(kv);
        } else if let Some(text) = self.text.as_mut() {
            text.handle_key_event(kv);
        } else {
            unreachable!()
        }
    }

    pub fn buffer() -> Self {
        Self {
            widget: WidgetState::Buffer {
                buffer: String::new(),
                cursor: 0,
            },
            text: None,
            focus_on_text: false,
        }
    }
    pub fn select(size: usize, is_single: bool) -> Self {
        Self {
            widget: WidgetState::Select {
                selected: vec![false; size],
                cursor: 0,
                is_single,
            },
            text: None,
            focus_on_text: false,
        }
    }
    pub fn with_prompt(mut self) -> Self {
        self.text = Some(TextState::default());
        self.focus_on_text = true;
        self
    }
}

#[derive(Default)]
pub struct TextState {
    vect_offset: u8,
    hori_offset: u8,

    vect_limit: u8,
    hori_limit: u8,
    page_size: u8,
}
impl TextState {
    fn handle_key_event(&mut self, kv: &Key) {
        match kv.code {
            KeyCode::Down => {}
            KeyCode::Up => {}
            KeyCode::Right => {}
            KeyCode::Left => {}

            KeyCode::PageDown => {}
            KeyCode::PageUp => {}

            KeyCode::Home => {}
            KeyCode::End => {}
            _ => {}
        }
    }
}

pub enum WidgetState {
    Buffer {
        buffer: String,
        cursor: usize,
    },
    Select {
        selected: Vec<bool>,
        cursor: usize,
        is_single: bool,
    },
}

impl WidgetState {
    fn handle_key_event(&mut self, kv: &KeyEvent) {
        match self {
            Self::Buffer { buffer, cursor } => {
                match kv.code {
                    KeyCode::Char(ch) => {}
                    KeyCode::Backspace => {}
                    KeyCode::Left => {}
                    KeyCode::Right => {}
                    _ => {}
                }
                todo!()
            }
            Self::Select {
                selected,
                cursor,
                is_single,
            } => {
                debug_assert!(selected.len() != 0);

                match kv.code {
                    KeyCode::Down if *cursor == 0 => {}
                    KeyCode::Down => *cursor -= 1,

                    KeyCode::Up if *cursor == selected.len() - 1 => {}
                    KeyCode::Up => *cursor += 1,

                    KeyCode::Char(' ') if !*is_single => {
                        let b = selected[*cursor];
                        selected[*cursor] = !b;
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn end_select_single(self) -> usize {
        let selected = self.end_select_multi();
        debug_assert_eq!(selected.len(), 1);
        selected[0]
    }
    pub fn end_select_multi(self) -> Vec<usize> {
        let Self::Select { selected, .. } = self else {
            unreachable!()
        };
        selected
            .into_iter()
            .enumerate()
            .filter_map(|(n, b)| b.then_some(n))
            .collect()
    }

    pub fn as_widget(&self) -> impl ratatui::widgets::Widget {
        use ratatui::style::{Modifier, Stylize};
        use ratatui::text::{Line, Span};

        match self {
            Self::Buffer { buffer, cursor } => {
                let mut before = format!("{} ", buffer);
                let mut after = before.split_off(*cursor);
                let cursor = after.remove(0);
                ratatui::widgets::Paragraph::new(Line::from_iter([
                    Span::raw("> "),
                    Span::raw(before),
                    Span::raw(cursor.to_string()).add_modifier(Modifier::REVERSED),
                    Span::raw(after),
                ]))
                // .scroll((0, (cursor as u16).saturating_sub(area.width - 8)))
            }
            Self::Select {
                selected,
                cursor,
                is_single,
            } => todo!(),
        }
    }
}
