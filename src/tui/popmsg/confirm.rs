use super::dev::*;
use ratatui::{style::Stylize, text::Span, widgets::Paragraph};

/// Used to display message without reply
pub struct Confirm {
    dismiss_on_any_key: bool,
    notification: bool,
}

impl Confirm {
    pub fn title(title: String) -> MsgBuilder<Self> {
        MsgBuilder::new(
            Self {
                dismiss_on_any_key: false,
                notification: false,
            },
            title,
        )
    }
    pub fn notification(title: String) -> MsgBuilder<Self> {
        MsgBuilder::new(
            Self {
                dismiss_on_any_key: false,
                notification: true,
            },
            title,
        )
    }
    pub fn dismiss_any(title: String) -> MsgBuilder<Self> {
        MsgBuilder::new(
            Self {
                dismiss_on_any_key: true,
                notification: false,
            },
            title,
        )
    }
    pub fn err(e: impl std::fmt::Display) {
        Self::notification("Error".to_owned())
            .with_prompt(e.to_string())
            .build_and_send();
    }

    pub async fn err_async(e: impl std::fmt::Display) {
        Self::notification("Error".to_owned())
            .with_prompt(e.to_string())
            .build_and_send_async()
            .await;
    }

    pub async fn message_async(title: String, prompt: String) {
        Self::notification(title)
            .with_prompt(prompt)
            .build_and_send_async()
            .await;
    }
}

impl Msg for Confirm {
    type Result = ();

    fn is_notification(&self) -> bool {
        self.notification
    }

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
