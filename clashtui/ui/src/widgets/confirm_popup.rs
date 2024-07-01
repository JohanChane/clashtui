use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::prelude as Ra;

use crate::{EventState, Infailable};

use super::MsgPopup;
/// Modified [MsgPopup]
///
/// Add 'y'/Enter, 'n'/Esc to close
///
/// NOTE: default work in confirm mode, call `ConfirmPopup::should_confirm`
///  to change the behavior
///
/// Not impl [Visibility][crate::Visibility] since [MsgPopup] does
#[derive(Default)]
pub struct ConfirmPopup(MsgPopup, bool);

impl ConfirmPopup {
    pub fn new() -> Self {
        Self(MsgPopup::default(), false)
    }

    pub fn should_confirm(&mut self, should: bool) {
        self.1 = should
    }

    pub fn event(&mut self, ev: &Event) -> Result<EventState, Infailable> {
        if !self.0.is_visible() {
            return Ok(EventState::NotConsumed);
        }

        let mut event_state = EventState::NotConsumed;
        if let Event::Key(key) = ev {
            if key.kind != KeyEventKind::Press {
                return Ok(EventState::NotConsumed);
            }
            match key.code {
                KeyCode::Char('y') | KeyCode::Enter if self.1 => {
                    self.0.hide();
                    return Ok(EventState::Yes);
                }
                KeyCode::Char('n') | KeyCode::Esc if self.1 => {
                    self.0.hide();
                    return Ok(EventState::Cancel);
                }
                _ => {
                    event_state = self.0.event(ev)?;
                }
            }
        }

        Ok(event_state)
    }

    pub fn draw(&mut self, f: &mut Ra::Frame, _area: Ra::Rect) {
        //! area is only used to keep the args
        self.0.draw(f, _area);
    }

    pub fn popup_msg(&mut self, confirm_str: String) {
        self.0.push_txt_msg(confirm_str);
        self.0.show();
    }
}
