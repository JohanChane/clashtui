use ratatui::{style::Stylize, text::Span, widgets::Paragraph};

use super::*;
use crossterm::event::KeyCode;

/// Used to display message without reply
pub struct Confirm {
    dismiss_on_any_key: bool,
}

impl Confirm {
    pub fn title(title: String) -> MsgBuilder<Self> {
        MsgBuilder::new(Self { dismiss_on_any_key: false }, title)
    }
    pub fn dismiss_any(title: String) -> MsgBuilder<Self> {
        MsgBuilder::new(Self { dismiss_on_any_key: true }, title)
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
        if self.dismiss_on_any_key {
            return Route::Send;
        }
        if matches!(kv.code, KeyCode::Enter | KeyCode::Char(' ')) {
            Route::Send
        } else if matches!(kv.code, KeyCode::Esc) {
            Route::Drop
        } else {
            Route::Keep
        }
    }

    fn send(self, tx: Sender<Self::Result>) {
        let _ = tx.send(());
    }

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
