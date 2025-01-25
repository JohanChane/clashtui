use crossterm::event::KeyCode;
use ratatui::prelude as Ra;
use ratatui::widgets as Raw;

use crate::tui::misc::EventState;
use crate::tui::{Drawable, Theme};

/// A single input widget like
///```md
/// ┌title─────────────────────────────┐
/// │content                           │
/// └──────────────────────────────────┘
///```
pub struct Item {
    buffer: String,
    cursor: usize,
    title: String,
}
impl Item {
    pub fn title(title: String) -> Self {
        Self {
            title,
            buffer: Default::default(),
            cursor: Default::default(),
        }
    }
    /// consume self and get content
    pub fn content(self) -> String {
        self.buffer
    }
}

impl Drawable for Item {
    fn render(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect, is_fouced: bool) {
        let page = Raw::Paragraph::new(self.buffer.as_str())
            .style(Ra::Style::default().fg(if is_fouced {
                Theme::get().input_text_selected_fg
            } else {
                Theme::get().input_text_unselected_fg
            }))
            .block(
                Raw::Block::default()
                    .borders(Raw::Borders::ALL)
                    .title(self.title.as_str()),
            );
        f.render_widget(page, area);
    }
    /// - chars/Left/Right/Backspace -> [EventState::WorkDone]
    /// - Enter -> [EventState::Yes]
    /// - Esc -> [EventState::Cancel]
    /// - unrecognized event -> [EventState::NotConsumed]
    fn handle_key_event(&mut self, ev: &crossterm::event::KeyEvent) -> EventState {
        match ev.code {
            KeyCode::Char(ch) => self.enter_char(ch),
            KeyCode::Backspace => self.delete_char(),
            KeyCode::Left => self.move_cursor_left(),
            KeyCode::Right => self.move_cursor_right(),

            KeyCode::Enter => {
                return EventState::Yes;
            }
            KeyCode::Esc => {
                return EventState::Cancel;
            }
            _ => return EventState::NotConsumed,
        }
        EventState::WorkDone
    }
}
impl Item {
    fn delete_char(&mut self) {
        if self.cursor != 0 {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.
            self.buffer = self
                .buffer
                .char_indices()
                .filter_map(|(idx, ch)| (idx != self.cursor - 1).then_some(ch))
                .collect();
            self.move_cursor_left();
        }
    }
    fn enter_char(&mut self, ch: char) {
        self.buffer.insert(self.cursor, ch);
        self.move_cursor_right();
    }
    fn move_cursor_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1).clamp(0, self.buffer.len());
    }

    fn move_cursor_right(&mut self) {
        self.cursor = self.cursor.saturating_add(1).clamp(0, self.buffer.len());
    }
}

#[cfg(test)]
mod test {
    use super::Item;
    use crate::tui::{misc::EventState, Drawable};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    trait Test {
        fn apply_test(
            self,
            ops: &[KeyCode],
            check_evsts: &[EventState],
            check_buffers: &[&str],
            check_cusors: &[usize],
        ) -> Self;
    }
    impl Test for Item {
        fn apply_test(
            mut self,
            ops: &[KeyCode],
            check_evsts: &[EventState],
            check_buffers: &[&str],
            check_cusors: &[usize],
        ) -> Self {
            for (((&op, &evst), &buffer), &cursor) in ops
                .into_iter()
                .zip(check_evsts)
                .zip(check_buffers)
                .zip(check_cusors)
            {
                let e = self.handle_key_event(&KeyEvent::new(op, KeyModifiers::empty()));
                assert_eq!(e, evst, "now running {op} {evst:?} {buffer} {cursor}");
                assert_eq!(
                    self.cursor, cursor,
                    "now running {op} {evst:?} {buffer} {cursor}"
                );
                assert_eq!(
                    self.buffer.as_str(),
                    buffer,
                    "now running {op} {evst:?} {buffer} {cursor}"
                )
            }
            self
        }
    }

    #[test]
    fn get_strs() {
        let it = Item::title("test".to_owned());
        assert_eq!(it.title.as_str(), "test");
        assert_eq!(it.cursor, 0);
        assert_eq!(it.buffer.as_str(), "");
        it.apply_test(
            &[KeyCode::Char('t'), KeyCode::Backspace, KeyCode::Char('t')],
            &[
                EventState::WorkDone,
                EventState::WorkDone,
                EventState::WorkDone,
            ],
            &["t", "", "t"],
            &[1, 0, 1],
        )
        .apply_test(
            &[KeyCode::Char('e'), KeyCode::Esc, KeyCode::Enter],
            &[EventState::WorkDone, EventState::Cancel, EventState::Yes],
            &["te", "te", "te"],
            &[2, 2, 2],
        )
        .apply_test(
            &[KeyCode::Char('s'), KeyCode::Left, KeyCode::Delete],
            &[
                EventState::WorkDone,
                EventState::WorkDone,
                EventState::NotConsumed,
            ],
            &["tes", "tes", "tes"],
            &[3, 2, 2],
        )
        .apply_test(
            &[KeyCode::Right, KeyCode::Char('t'), KeyCode::Right],
            &[
                EventState::WorkDone,
                EventState::WorkDone,
                EventState::WorkDone,
            ],
            &["tes", "test", "test"],
            &[3, 4, 4],
        );
    }
}
