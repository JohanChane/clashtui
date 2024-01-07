use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{prelude::*, widgets::*};

use crate::ui::popups::ClashTuiInputPopup;
use crate::ui::EventState;

pub struct ProfileInputPopup {
    pub name_input: ClashTuiInputPopup,
    pub uri_input: ClashTuiInputPopup,
}

impl ProfileInputPopup {
    pub fn new() -> Self {
        let mut obj = Self {
            name_input: ClashTuiInputPopup::new("Name".to_string()),
            uri_input: ClashTuiInputPopup::new("Uri".to_string()),
        };
        obj.name_input.set_fouce(true);
        obj.uri_input.set_fouce(false);

        obj
    }

    pub fn event(&mut self, ev: &Event) -> Result<EventState> {
        if !self.is_visible() {
            return Ok(EventState::NotConsumed);
        }

        let mut event_state = EventState::NotConsumed;
        if let Event::Key(key) = ev {
            if key.kind == KeyEventKind::Press {
                event_state = match key.code {
                    KeyCode::Tab => {
                        self.switch_fouce();
                        EventState::WorkDone
                    }
                    KeyCode::Enter => {
                        self.name_input.handle_enter_ev();
                        self.uri_input.handle_enter_ev();
                        self.hide();

                        EventState::WorkDone
                    }
                    KeyCode::Esc => {
                        self.name_input.handle_esc_ev();
                        self.uri_input.handle_esc_ev();
                        self.hide();

                        EventState::WorkDone
                    }
                    _ => {
                        event_state = self.name_input.event(ev)?;
                        if event_state.is_notconsumed() {
                            event_state = self.uri_input.event(ev)?;
                        }
                        event_state
                    }
                };
            }
        }

        Ok(event_state)
    }

    pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        if !self.is_visible() {
            return;
        }

        let chunks = Layout::default()
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .margin(1)
            .split(area);

        f.render_widget(Clear, area);

        self.name_input.draw(f, chunks[0]);
        self.uri_input.draw(f, chunks[1]);

        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(135, 206, 236)))
            .title("InputProfile");
        f.render_widget(block, area);
    }

    pub fn switch_fouce(&mut self) {
        if self.name_input.is_fouce() {
            self.name_input.set_fouce(false);
            self.uri_input.set_fouce(true);
        } else {
            self.name_input.set_fouce(true);
            self.uri_input.set_fouce(false);
        }
    }

    pub fn is_visible(&self) -> bool {
        self.name_input.is_visible() && self.uri_input.is_visible()
    }
    pub fn show(&mut self) {
        self.name_input.show();
        self.uri_input.show();
        self.name_input.set_fouce(true);
        self.uri_input.set_fouce(false);
    }
    pub fn hide(&mut self) {
        self.name_input.hide();
        self.uri_input.hide();
    }
}
