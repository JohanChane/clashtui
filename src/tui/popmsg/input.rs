use super::dev::*;
use ratatui::{
    style::Stylize as _,
    text::{Line, Span},
    widgets::Paragraph,
};
use unicode_width::UnicodeWidthStr;

#[derive(Default)]
pub struct Input {
    buffer: String,
    cursor: usize,
}

impl Msg for Input {
    type Result = String;

    fn match_key_event(&mut self, kv: &Key) -> Route {
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
            let byte_pos = self.byte_offset();
            let mut before = format!("{} ", self.buffer);
            let mut after = before.split_off(byte_pos);
            let cursor = after.remove(0);
            let prefix_width = UnicodeWidthStr::width(&self.buffer[..byte_pos]) as u16;
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
            .scroll((0, prefix_width.saturating_sub(area.width.saturating_sub(8))))
        };
        f.render_widget(widget, area);
    }

    fn size(&self) -> (u16, u16) {
        let width = UnicodeWidthStr::width(self.buffer.as_str()).max(10) as u16;
        (width + 2, 1)
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
    fn byte_offset(&self) -> usize {
        self.buffer
            .char_indices()
            .nth(self.cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.buffer.len())
    }

    fn delete_char(&mut self) {
        if self.cursor != 0 {
            self.buffer = self
                .buffer
                .char_indices()
                .enumerate()
                .filter_map(|(pos, (_, ch))| (pos != self.cursor - 1).then_some(ch))
                .collect();
            self.cursor = self.cursor.saturating_sub(1);
        }
    }
    fn delete_char_inplace(&mut self) {
        self.buffer = self
            .buffer
            .char_indices()
            .enumerate()
            .filter_map(|(pos, (_, ch))| (pos != self.cursor).then_some(ch))
            .collect();
        self.cursor = self.cursor.saturating_sub(1);
    }
    fn enter_char(&mut self, ch: char) {
        let pos = self.byte_offset();
        self.buffer.insert(pos, ch);
        self.cursor = self.cursor.saturating_add(1);
    }
    fn move_cursor_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }
    fn move_cursor_right(&mut self) {
        self.cursor = self
            .cursor
            .saturating_add(1)
            .min(self.buffer.chars().count());
    }
}

#[derive(Default)]
pub struct InputMasked {
    buffer: String,
    cursor: usize,
}

impl Msg for InputMasked {
    type Result = String;

    fn match_key_event(&mut self, kv: &Key) -> Route {
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
            let byte_pos = self.byte_offset();
            let mut before = format!("{} ", masked);
            let mut after = before.split_off(byte_pos);
            let cursor = after.remove(0);
            let prefix_width = byte_pos as u16;
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
            .scroll((0, prefix_width.saturating_sub(area.width.saturating_sub(8))))
        };
        f.render_widget(widget, area);
    }

    fn size(&self) -> (u16, u16) {
        let width = self.buffer.len().max(10) as u16;
        (width + 2, 1)
    }
}

impl InputMasked {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn with_title(self, title: String) -> MsgBuilder<Self> {
        MsgBuilder::new(self, title)
    }

    fn byte_offset(&self) -> usize {
        self.buffer
            .char_indices()
            .nth(self.cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.buffer.len())
    }

    fn delete_char(&mut self) {
        if self.cursor != 0 {
            self.buffer = self
                .buffer
                .char_indices()
                .enumerate()
                .filter_map(|(pos, (_, ch))| (pos != self.cursor - 1).then_some(ch))
                .collect();
            self.cursor = self.cursor.saturating_sub(1);
        }
    }
    fn delete_char_inplace(&mut self) {
        self.buffer = self
            .buffer
            .char_indices()
            .enumerate()
            .filter_map(|(pos, (_, ch))| (pos != self.cursor).then_some(ch))
            .collect();
        self.cursor = self.cursor.saturating_sub(1);
    }
    fn enter_char(&mut self, ch: char) {
        let pos = self.byte_offset();
        self.buffer.insert(pos, ch);
        self.cursor = self.cursor.saturating_add(1);
    }
    fn move_cursor_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }
    fn move_cursor_right(&mut self) {
        self.cursor = self
            .cursor
            .saturating_add(1)
            .min(self.buffer.chars().count());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_cjk_insert_between_chars() {
        let mut inp = Input {
            buffer: "你好".into(),
            cursor: 1,
        };
        inp.enter_char('中');
        assert_eq!(inp.buffer, "你中好");
        assert_eq!(inp.cursor, 2);
    }

    #[test]
    fn input_cjk_insert_at_beginning() {
        let mut inp = Input {
            buffer: "你好".into(),
            cursor: 0,
        };
        inp.enter_char('啊');
        assert_eq!(inp.buffer, "啊你好");
        assert_eq!(inp.cursor, 1);
    }

    #[test]
    fn input_cjk_insert_at_end() {
        let mut inp = Input {
            buffer: "你好".into(),
            cursor: 2,
        };
        inp.enter_char('啊');
        assert_eq!(inp.buffer, "你好啊");
        assert_eq!(inp.cursor, 3);
    }

    #[test]
    fn input_move_cursor_right_with_cjk() {
        let mut inp = Input {
            buffer: "你好".into(),
            cursor: 1,
        };
        inp.move_cursor_right();
        assert_eq!(inp.cursor, 2);
        inp.move_cursor_right();
        assert_eq!(inp.cursor, 2);
    }

    #[test]
    fn input_cjk_delete_char() {
        let mut inp = Input {
            buffer: "你中好".into(),
            cursor: 2,
        };
        inp.delete_char();
        assert_eq!(inp.buffer, "你好");
        assert_eq!(inp.cursor, 1);
    }

    #[test]
    fn masked_cjk_insert_between_chars() {
        let mut inp = InputMasked {
            buffer: "你好".into(),
            cursor: 1,
        };
        inp.enter_char('中');
        assert_eq!(inp.buffer, "你中好");
        assert_eq!(inp.cursor, 2);
    }

    #[test]
    fn masked_move_cursor_right_with_cjk() {
        let mut inp = InputMasked {
            buffer: "你好".into(),
            cursor: 1,
        };
        inp.move_cursor_right();
        assert_eq!(inp.cursor, 2);
        inp.move_cursor_right();
        assert_eq!(inp.cursor, 2);
    }
}
