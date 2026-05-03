use super::dev::*;
use ratatui::{
    style::Stylize as _,
    text::{Line, Span},
    widgets::Paragraph,
};

#[derive(Default)]
pub struct Input {
    buffer: String,
    cursor: usize,
}

impl Msg for Input {
    type Result = String;

    fn match_key_event(&mut self, kv: &KeyEvent) -> Route {
        match kv.code {
            KeyCode::Enter => return Route::Send,
            KeyCode::Esc => return Route::Drop,

            KeyCode::Char(ch) => self.enter_char(ch),
            KeyCode::Backspace => self.delete_char(),
            KeyCode::Delete => self.delete_char_inplace(),
            KeyCode::Left => self.move_cursor_left(),
            KeyCode::Right => self.move_cursor_right(),
            _ => {}
        }
        Route::Keep
    }

    fn send(self, tx: Sender<Self::Result>) {
        tx.send(self.buffer).unwrap()
    }

    fn render(&self, f: &mut Frame, area: Rect, block: Block, is_focused: bool) {
        let widget = {
            let mut before = format!("{} ", self.buffer);
            let mut after = before.split_off(self.cursor);
            let cursor = after.remove(0);
            Paragraph::new(Line::from_iter([
                Span::raw("> "),
                Span::raw(before),
                if is_focused {
                    Span::raw(cursor.to_string()).reversed()
                } else {
                    Span::raw(cursor.to_string())
                },
                Span::raw(after),
            ]))
            .block(block)
            .scroll((0, (self.cursor as u16).saturating_sub(area.width - 8)))
        };
        f.render_widget(widget, area);
    }

    fn size(&self) -> (u16, u16) {
        (self.buffer.len().max(10) as u16 + 2, 1)
    }
}

impl Input {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn with_title(self, title: String) -> MsgBuilder<Self> {
        MsgBuilder::new(self, title)
    }
}

impl Input {
    fn delete_char(&mut self) {
        if self.cursor != 0 {
            self.buffer = self
                .buffer
                .char_indices()
                .filter_map(|(idx, ch)| (idx != self.cursor - 1).then_some(ch))
                .collect();
            self.cursor = self.cursor.saturating_sub(1);
        }
    }
    fn delete_char_inplace(&mut self) {
        self.buffer = self
            .buffer
            .char_indices()
            .filter_map(|(idx, ch)| (idx != self.cursor).then_some(ch))
            .collect();
        self.cursor = self.cursor.saturating_sub(1);
    }
    fn enter_char(&mut self, ch: char) {
        self.buffer.insert(self.cursor, ch);
        self.cursor = self.cursor.saturating_add(1).min(self.buffer.len());
    }
    fn move_cursor_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }
    fn move_cursor_right(&mut self) {
        self.cursor = self.cursor.saturating_add(1).min(self.buffer.len());
    }
}

#[derive(Default)]
pub struct InputMasked {
    buffer: String,
    cursor: usize,
}

impl Msg for InputMasked {
    type Result = String;

    fn match_key_event(&mut self, kv: &KeyEvent) -> Route {
        match kv.code {
            KeyCode::Enter => return Route::Send,
            KeyCode::Esc => return Route::Drop,

            KeyCode::Char(ch) => self.enter_char(ch),
            KeyCode::Backspace => self.delete_char(),
            KeyCode::Delete => self.delete_char_inplace(),
            KeyCode::Left => self.move_cursor_left(),
            KeyCode::Right => self.move_cursor_right(),
            _ => {}
        }
        Route::Keep
    }

    fn send(self, tx: Sender<Self::Result>) {
        tx.send(self.buffer).unwrap()
    }

    fn render(&self, f: &mut Frame, area: Rect, block: Block, is_focused: bool) {
        let mask_len = self.buffer.len();
        let masked: String = "*".repeat(mask_len);
        let widget = {
            let mut before = format!("{} ", masked);
            let mut after = before.split_off(self.cursor);
            let cursor = after.remove(0);
            Paragraph::new(Line::from_iter([
                Span::raw("> "),
                Span::raw(before),
                if is_focused {
                    Span::raw(cursor.to_string()).reversed()
                } else {
                    Span::raw(cursor.to_string())
                },
                Span::raw(after),
            ]))
            .block(block)
            .scroll((
                0,
                (self.cursor as u16).saturating_sub(area.width.saturating_sub(8)),
            ))
        };
        f.render_widget(widget, area);
    }

    fn size(&self) -> (u16, u16) {
        (self.buffer.len().max(10) as u16 + 2, 1)
    }
}

impl InputMasked {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn with_title(self, title: String) -> MsgBuilder<Self> {
        MsgBuilder::new(self, title)
    }
    fn delete_char(&mut self) {
        if self.cursor != 0 {
            self.buffer = self
                .buffer
                .char_indices()
                .filter_map(|(idx, ch)| (idx != self.cursor - 1).then_some(ch))
                .collect();
            self.cursor = self.cursor.saturating_sub(1);
        }
    }
    fn delete_char_inplace(&mut self) {
        self.buffer = self
            .buffer
            .char_indices()
            .filter_map(|(idx, ch)| (idx != self.cursor).then_some(ch))
            .collect();
        self.cursor = self.cursor.saturating_sub(1);
    }
    fn enter_char(&mut self, ch: char) {
        self.buffer.insert(self.cursor, ch);
        self.cursor = self.cursor.saturating_add(1).min(self.buffer.len());
    }
    fn move_cursor_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }
    fn move_cursor_right(&mut self) {
        self.cursor = self.cursor.saturating_add(1).min(self.buffer.len());
    }
}
