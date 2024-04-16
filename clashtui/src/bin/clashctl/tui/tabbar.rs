use ratatui::{prelude as Ra, widgets as Raw};
use ui::event::{Event, KeyCode, KeyEventKind};

use super::Theme;
use crate::tui::EventState;

pub struct TabBar {
    is_visible: bool,
    tab_titles: Vec<String>,
    index: usize,
}
impl TabBar {
    pub fn new(tab_titles: Vec<String>) -> Self {
        Self {
            is_visible: true,
            tab_titles,
            index: 0,
        }
    }

    pub fn event(&mut self, ev: &Event) -> Result<EventState, ()> {
        if !self.is_visible {
            return Ok(EventState::NotConsumed);
        }

        let mut event_stata = EventState::NotConsumed;
        if let Event::Key(key) = ev {
            if key.kind == KeyEventKind::Press {
                event_stata = match key.code {
                    // 1..=9
                    // need to kown the range
                    #[allow(clippy::is_digit_ascii_radix)]
                    KeyCode::Char(c) if c.is_digit(10) && c != '0' => {
                        let digit = c.to_digit(10);
                        if let Some(d) = digit {
                            if d <= self.tab_titles.len() as u32 {
                                self.index = (d - 1) as usize;
                            }
                        }
                        EventState::WorkDone
                    }
                    KeyCode::Tab => {
                        self.next();
                        EventState::WorkDone
                    }
                    _ => EventState::NotConsumed,
                }
            }
        }

        Ok(event_stata)
    }

    pub fn draw(&mut self, f: &mut Ra::Frame, area: Ra::Rect) {
        let items: Vec<Ra::text::Line> = self
            .tab_titles
            .iter()
            .map(|t| {
                Ra::text::Line::from(Ra::Span::styled(
                    t,
                    Ra::Style::default().fg(Theme::get().tabbar_text_fg),
                ))
            })
            .collect();
        let tabs = Raw::Tabs::new(items)
            .block(Raw::Block::default().borders(Raw::Borders::ALL))
            .highlight_style(Ra::Style::default().fg(Theme::get().tabbar_hl_fg))
            .select(self.index);
        f.render_widget(tabs, area);
    }

    pub fn next(&mut self) {
        self.index = (self.index + 1) % self.tab_titles.len();
    }
    #[allow(unused)]
    pub fn previous(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        } else {
            self.index = self.tab_titles.len() - 1;
        }
    }

    pub fn selected(&self) -> Option<&String> {
        self.tab_titles.get(self.index)
    }
}
