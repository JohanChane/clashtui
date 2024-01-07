use anyhow::Result;
use crossterm::event::{Event, KeyEventKind};
use ratatui::prelude::Backend;
use ratatui::style::Style;
use ratatui::{
    prelude::{Frame, Rect, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::utils::SharedClashTuiState;
use crate::ui::{widgets::SharedTheme, EventState};
use crate::visible_methods;

pub struct ClashTuiStatusBar {
    title: String,
    is_visible: bool,
    clashtui_state: SharedClashTuiState,

    theme: SharedTheme,
}

impl ClashTuiStatusBar {
    pub fn new(clashtui_state: SharedClashTuiState, theme: SharedTheme) -> Self {
    Self {
            title: "StatusBar".to_string(),
            is_visible: true,
            clashtui_state,

            theme,
        }
    }

    pub fn event(&mut self, ev: &Event) -> Result<EventState> {
        if !self.is_visible {
            return Ok(EventState::NotConsumed);
        }

        let mut event_state = EventState::NotConsumed;
        if let Event::Key(key) = ev {
            if key.kind == KeyEventKind::Press {
                event_state = match key.code {
                    _ => EventState::NotConsumed,
                };
            }
        }

        Ok(event_state)
    }

    pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        if !self.is_visible {
            return;
        }

        f.render_widget(Clear, area);
        #[cfg(target_os = "windows")]
        let status_str = format!(
                "Profile: {}    Tun: {}    SysProxy: {}    Help: ?",
                self.clashtui_state.borrow().get_profile(),
                self.clashtui_state.borrow().get_tun(),
                self.clashtui_state.borrow().get_sysproxy().to_string(),
            );
        #[cfg(target_os = "linux")]
        let status_str = format!(
                "Profile: {}    Tun: {}    Help: ?",
                self.clashtui_state.borrow().get_profile(),
                self.clashtui_state.borrow().get_tun(),
            );
        
        let paragraph = Paragraph::new(Span::styled(
            status_str,
            Style::default().fg(self.theme.statusbar_txt_fg),
        ))
        //.alignment(ratatui::prelude::Alignment::Right)
        .wrap(Wrap { trim: true });
        let block = Block::new().borders(Borders::ALL);
        f.render_widget(paragraph.clone().block(block), area);
    }
}

visible_methods!(ClashTuiStatusBar);
