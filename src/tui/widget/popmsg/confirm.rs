use ratatui::{style::Stylize, text::Span, widgets::Paragraph};

use super::*;
use crossterm::event::KeyCode;

/// Used to display message without reply
pub struct Confirm;

impl Confirm {
    pub fn title(title: String) -> MsgBuilder<Self> {
        MsgBuilder::new(Self, title)
    }
    pub fn err(e: impl std::fmt::Display) {
        Self::title("Error".to_owned())
            .with_prompt(e.to_string())
            .build_and_send();
    }
}

impl Msg for Confirm {
    type Result = ();

    fn match_key_event(&mut self, kv: &Key) -> Route {
        if matches!(kv.code, KeyCode::Enter | KeyCode::Esc | KeyCode::Char(' ')) {
            Route::Drop
        } else {
            Route::Keep
        }
    }

    fn send(self, _: Sender<Self::Result>) {}

    fn render(&self, f: &mut Frame, area: Rect, block: Block, is_focused: bool) {
        let widget = Paragraph::new(if is_focused {
            Span::raw("OK").reversed()
        } else {
            Span::raw("OK")
        })
        .centered()
        .block(block);
        f.render_widget(widget, area);
    }

    fn size(&self) -> (u16, u16) {
        (20, 1)
    }
}
