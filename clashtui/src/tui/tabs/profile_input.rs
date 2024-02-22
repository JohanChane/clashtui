use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{prelude as Ra, widgets as Raw};

use crate::tui::{widgets::InputPopup, EventState, Visibility};

#[derive(PartialEq)]
enum Fouce {
    Name,
    Uri,
}

pub struct ProfileInputPopup {
    pub name_input: InputPopup,
    pub uri_input: InputPopup,
    fouce: Fouce,
}

impl ProfileInputPopup {
    pub fn new() -> Self {
        Self {
            name_input: InputPopup::new("Name".to_string()),
            uri_input: InputPopup::new("Uri".to_string()),
            fouce: Fouce::Name,
        }
    }

    pub fn event(&mut self, ev: &Event) -> Result<EventState, ui::Infailable> {
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
                    _ => match self.fouce {
                        Fouce::Name => self.name_input.event(ev)?,
                        Fouce::Uri => self.uri_input.event(ev)?,
                    },
                };
            }
        }

        Ok(event_state)
    }

    pub fn draw(&mut self, f: &mut Ra::Frame, area: Ra::Rect) {
        //! will clear the area
        if !self.is_visible() {
            return;
        }

        let chunks = Ra::Layout::default()
            .constraints([
                Ra::Constraint::Percentage(50),
                Ra::Constraint::Percentage(50),
            ])
            .margin(1)
            .split(area);

        f.render_widget(Raw::Clear, area);
        let sel = self.fouce == Fouce::Name;
        self.name_input.draw(f, chunks[0], sel);
        self.uri_input.draw(f, chunks[1], !sel);

        let block = Raw::Block::new()
            .borders(Raw::Borders::ALL)
            .border_style(Ra::Style::default().fg(Ra::Color::Rgb(135, 206, 236)))
            .title("InputProfile");
        f.render_widget(block, area);
    }

    pub fn switch_fouce(&mut self) {
        if self.fouce == Fouce::Name {
            self.fouce = Fouce::Uri;
        } else {
            self.fouce = Fouce::Name;
        }
    }

    pub fn is_visible(&self) -> bool {
        self.name_input.is_visible() && self.uri_input.is_visible()
    }
    pub fn show(&mut self) {
        self.fouce = Fouce::Name;
        self.name_input.show();
        self.uri_input.show();
    }
    pub fn hide(&mut self) {
        self.name_input.hide();
        self.uri_input.hide();
    }
}
