use ratatui::{prelude as Ra, widgets as Raw};

use super::SharedTheme;
use crate::utils::SharedClashTuiState;

pub struct StatusBar {
    is_visible: bool,
    clashtui_state: SharedClashTuiState,

    theme: SharedTheme,
}

impl StatusBar {
    pub fn new(clashtui_state: SharedClashTuiState, theme: SharedTheme) -> Self {
        Self {
            is_visible: true,
            clashtui_state,
            theme,
        }
    }

    pub fn draw(&mut self, f: &mut Ra::Frame, area: Ra::Rect) {
        if !self.is_visible {
            return;
        }

        f.render_widget(Raw::Clear, area);
        let state = self.clashtui_state.borrow();
        let status_str = state.render();
        let paragraph = Raw::Paragraph::new(Ra::Span::styled(
            status_str,
            Ra::Style::default().fg(self.theme.statusbar_txt_fg),
        ))
        //.alignment(ratatui::prelude::Alignment::Right)
        .wrap(Raw::Wrap { trim: true });
        let block = Raw::Block::new().borders(Raw::Borders::ALL);
        f.render_widget(paragraph.block(block), area);
    }
}
