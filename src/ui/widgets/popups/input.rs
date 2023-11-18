use anyhow::{bail, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{prelude::*, widgets::*};

use crate::ui::EventState;
use crate::{fouce_methods, visible_methods};

pub struct ClashTuiInputPopup {
    title: String,
    is_visible: bool,
    is_fouce: bool,
    input: String,
    cursor_position: usize,
    input_data: String,
}

impl ClashTuiInputPopup {
    pub fn new(title: String) -> Self {
        Self {
            title,
            is_visible: false,
            is_fouce: true,
            input: String::new(),
            cursor_position: 0,
            input_data: String::new(),
        }
    }

    pub fn event(&mut self, ev: &Event) -> Result<EventState> {
        if !self.is_visible || !self.is_fouce {
            return Ok(EventState::NotConsumed);
        }

        if let Event::Key(key) = ev {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char(to_insert) => {
                        self.enter_char(to_insert);
                    }
                    KeyCode::Backspace => {
                        self.delete_char();
                    }
                    KeyCode::Left => {
                        self.move_cursor_left();
                    }
                    KeyCode::Right => {
                        self.move_cursor_right();
                    }
                    KeyCode::Enter => {
                        self.hide();
                        self.handle_enter_ev();
                    }
                    KeyCode::Esc => {
                        self.hide();
                        self.handle_esc_ev();
                    }
                    _ => {}
                };
            }
        }

        Ok(EventState::WorkDone)
    }

    pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        if !self.is_visible {
            return;
        }

        f.render_widget(Clear, area);

        let input = Paragraph::new(self.input.as_str())
            .style(Style::default().fg(if self.is_fouce {
                Color::Yellow
            } else {
                Color::default()
            }))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(self.title.clone()),
            );
        f.render_widget(input, area);
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.cursor_position.saturating_sub(1);
        self.cursor_position = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.cursor_position.saturating_add(1);
        self.cursor_position = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_char(&mut self, new_char: char) {
        self.input.insert(self.cursor_position, new_char);

        self.move_cursor_right();
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.cursor_position != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.cursor_position;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.len())
    }

    fn reset_cursor(&mut self) {
        self.cursor_position = 0;
    }

    fn submit_message(&mut self) {
        self.input_data = self.input.clone();
        self.input.clear();
        self.reset_cursor();
    }

    pub fn get_input_data(&self) -> String {
        self.input_data.clone()
    }

    pub fn handle_enter_ev(&mut self) {
        self.submit_message();
    }
    pub fn handle_esc_ev(&mut self) {
        self.input.clear();
        self.input_data.clear();
        self.reset_cursor();
    }
}

visible_methods!(ClashTuiInputPopup);
fouce_methods!(ClashTuiInputPopup);
