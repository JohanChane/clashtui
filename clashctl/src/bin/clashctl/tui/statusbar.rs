use ratatui::{prelude as Ra, widgets as Raw};

use super::Theme;
use crate::utils::SharedState;

pub struct StatusBar {
    is_visible: bool,
    state: SharedState,
}

impl StatusBar {
    pub fn new(state: SharedState) -> Self {
        Self {
            is_visible: true,
            state,
        }
    }

    pub fn draw(&mut self, f: &mut Ra::Frame, area: Ra::Rect) {
        if !self.is_visible {
            return;
        }

        f.render_widget(Raw::Clear, area);
        let state = self.state.borrow();
        let status_str = state.render();
        let paragraph = Raw::Paragraph::new(Ra::Span::styled(
            status_str,
            Ra::Style::default().fg(Theme::get().statusbar_text_fg),
        ))
        //.alignment(ratatui::prelude::Alignment::Right)
        .wrap(Raw::Wrap { trim: true });
        let block = Raw::Block::new().borders(Raw::Borders::ALL);
        f.render_widget(paragraph.block(block), area);
    }
}
