use crossterm::event::KeyCode;
use ratatui::{prelude as Ra, widgets as Raw};

use crate::tui::{misc::EventState, Drawable, Theme};

#[derive(PartialEq)]
enum Focus {
    Name,
    Url,
}

pub struct InputPopup {
    name_input: String,
    name_cursor: usize,
    url_input: String,
    url_cursor: usize,
    focus: Focus,
}

impl Drawable for InputPopup {
    /// No need to [Raw::clear], or plan aera
    fn render(&mut self, f: &mut ratatui::Frame, _: ratatui::layout::Rect, _: bool) {
        use Ra::{Constraint, Layout};
        let input_area = Layout::default()
            .constraints([
                Constraint::Percentage(25),
                Constraint::Length(8),
                Constraint::Min(0),
            ])
            .horizontal_margin(10)
            .vertical_margin(1)
            .split(f.area())[1];
        f.render_widget(Raw::Clear, input_area);
        let chunks = Ra::Layout::default()
            .constraints([
                Ra::Constraint::Percentage(50),
                Ra::Constraint::Percentage(50),
            ])
            .margin(1)
            .split(input_area);

        fn render_single(
            input: &str,
            f: &mut ratatui::Frame,
            area: Ra::Rect,
            is_selected: bool,
            title: &str,
        ) {
            let input = Raw::Paragraph::new(input)
                .style(Ra::Style::default().fg(if is_selected {
                    Theme::get().input_text_selected_fg
                } else {
                    Theme::get().input_text_unselected_fg
                }))
                .block(
                    Raw::Block::default()
                        .borders(Raw::Borders::ALL)
                        .title(title),
                );
            f.render_widget(input, area);
        }
        render_single(
            &self.name_input,
            f,
            chunks[0],
            self.focus == Focus::Name,
            "Name",
        );
        render_single(
            &self.url_input,
            f,
            chunks[1],
            self.focus == Focus::Url,
            "Url",
        );
        let block = Raw::Block::new()
            .borders(Raw::Borders::ALL)
            .border_style(Ra::Style::default().fg(Ra::Color::Rgb(135, 206, 236)))
            .title("Input");
        f.render_widget(block, input_area);
    }

    fn handle_key_event(
        &mut self,
        ev: &crossterm::event::KeyEvent,
    ) -> crate::tui::misc::EventState {
        if ev.kind != crossterm::event::KeyEventKind::Press {
            return EventState::NotConsumed;
        }
        match ev.code {
            KeyCode::Char(ch) => self.enter_char(ch),
            KeyCode::Backspace => self.delete_char(),
            KeyCode::Left => self.move_cursor_left(),
            KeyCode::Right => self.move_cursor_right(),

            KeyCode::Up | KeyCode::Down => self.switch_focus(),
            KeyCode::Enter => {
                self.reset_cursor();
                return EventState::Yes;
            }
            KeyCode::Esc => {
                self.reset_cursor();
                return EventState::Cancel;
            }
            _ => return EventState::NotConsumed,
        }
        EventState::WorkDone
    }
}

impl InputPopup {
    pub fn new() -> Self {
        Self {
            name_input: String::new(),
            name_cursor: 0,
            url_input: String::new(),
            url_cursor: 0,
            focus: Focus::Name,
        }
    }

    fn switch_focus(&mut self) {
        if self.focus == Focus::Name {
            self.focus = Focus::Url;
        } else {
            self.focus = Focus::Name;
        }
    }

    pub fn get_name_url(&mut self) -> (String, String) {
        let (n, u) = (self.name_input.clone(), self.url_input.clone());
        self.name_input.clear();
        self.url_input.clear();
        (n, u)
    }
}

impl Default for InputPopup {
    fn default() -> Self {
        Self::new()
    }
}

impl InputPopup {
    fn move_cursor_left(&mut self) {
        match self.focus {
            Focus::Name => move_cursor_left(&mut self.name_cursor, self.name_input.len()),
            Focus::Url => move_cursor_left(&mut self.url_cursor, self.url_input.len()),
        };
    }

    fn move_cursor_right(&mut self) {
        fn move_cursor_right(cursor: &mut usize, length: usize) {
            let cursor_moved_left = cursor.saturating_add(1);
            *cursor = cursor_moved_left.clamp(0, length);
        }
        match self.focus {
            Focus::Name => move_cursor_right(&mut self.name_cursor, self.name_input.len()),
            Focus::Url => move_cursor_right(&mut self.url_cursor, self.url_input.len()),
        };
    }

    fn enter_char(&mut self, ch: char) {
        match self.focus {
            Focus::Name => self.name_input.insert(self.name_cursor, ch),
            Focus::Url => self.url_input.insert(self.url_cursor, ch),
        };
        self.move_cursor_right();
    }

    fn delete_char(&mut self) {
        fn delete_char(cursor: &mut usize, input: &mut String) {
            if *cursor != 0 {
                delete_char_from_string(input, *cursor);
                move_cursor_left(cursor, input.len());
            }
        }
        match self.focus {
            Focus::Name => delete_char(&mut self.name_cursor, &mut self.name_input),
            Focus::Url => delete_char(&mut self.url_cursor, &mut self.url_input),
        };
    }

    fn reset_cursor(&mut self) {
        self.name_cursor = 0;
        self.url_cursor = 0;
    }
}

fn move_cursor_left(cursor: &mut usize, length: usize) {
    let cursor_moved_left = cursor.saturating_sub(1);
    *cursor = cursor_moved_left.clamp(0, length);
}

fn delete_char_from_string(input: &mut String, cursor: usize) {
    // Method "remove" is not used on the saved text for deleting the selected char.
    // Reason: Using remove on String works on bytes instead of the chars.
    // Using remove would require special care because of char boundaries.

    let from_left_to_current_index = cursor - 1;

    // Getting all characters before the selected character.
    let before_char_to_delete = input.chars().take(from_left_to_current_index);
    // Getting all characters after selected character.
    let after_char_to_delete = input.chars().skip(cursor);

    // Put all characters together except the selected one.
    // By leaving the selected one out, it is forgotten and therefore deleted.
    *input = before_char_to_delete.chain(after_char_to_delete).collect();
}
#[cfg(test)]
mod test {
    use super::delete_char_from_string;
    #[test]
    fn handle_delete_ascii() {
        let mut s = String::from("abc");
        assert!(s.is_ascii());
        let mut s2 = s.clone();
        s.remove(1);
        delete_char_from_string(&mut s2, 1 + 1);
        assert_eq!(s, *"ac");
        assert_eq!(s, s2);
    }
    #[test]
    #[should_panic = "byte index 1 is not a char boundary; it is inside '一' (bytes 0..3) of `一二三`"]
    fn handle_delete_utf_8() {
        let mut s = String::from("一二三");
        assert!(!s.is_ascii());
        let mut s2 = s.clone();
        delete_char_from_string(&mut s2, 1 + 1);
        assert_eq!(s2, *"一三");
        // panic here, utf-8 require more space
        // and a single index is not enough
        s.remove(1);
        assert_eq!(s, s2);
    }
    #[test]
    fn handle_insert_ascii() {
        let mut s = String::from("abc");
        assert!(s.is_ascii());
        s.insert(2, 'd');
        assert_eq!(s, *"abdc");
    }
    #[test]
    #[should_panic = "assertion failed: self.is_char_boundary(idx)"]
    fn handle_insert_utf_8() {
        let mut s = String::from("一二三");
        assert!(!s.is_ascii());
        // panic here, the reason is same
        s.insert(2, '四');
        assert_eq!(s, *"一二四三");
    }
    #[test]
    fn handle_append_by_insert() {
        let mut s = String::from("abc");
        let mut l = s.len();
        assert!(s.is_ascii());
        assert_eq!(l, 3);
        s.insert(l, 'd');
        l += 1;
        assert_eq!(s, *"abcd");
        assert_eq!(l, 4);
    }
}
