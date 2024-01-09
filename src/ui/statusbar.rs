use ratatui::{prelude as Ra, widgets as Raw};

use crate::ui::utils::SharedTheme;
use crate::utils::SharedClashTuiState;
// use crate::visible_methods;

pub struct ClashTuiStatusBar {
    is_visible: bool,
    clashtui_state: SharedClashTuiState,

    theme: SharedTheme,
}

impl ClashTuiStatusBar {
    pub fn new(clashtui_state: SharedClashTuiState, theme: SharedTheme) -> Self {
        Self {
            is_visible: true,
            clashtui_state,
            theme,
        }
    }

    pub fn draw<B: Ra::Backend>(&mut self, f: &mut Ra::Frame<B>, area: Ra::Rect) {
        if !self.is_visible {
            return;
        }

        f.render_widget(Raw::Clear, area);
        let state = self.clashtui_state.borrow();
        #[cfg(target_os = "windows")]
        let status_str = format!(
            "Profile: {}    Tun: {}    SysProxy: {}    Help: ?",
            state.get_profile(),
            state.get_tun(),
            state.get_sysproxy().to_string(),
        );
        #[cfg(target_os = "linux")]
        let status_str = format!(
            "Profile: {}    Tun: {}    Help: ?",
            state.get_profile(),
            state.get_tun(),
        );

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

// visible_methods!(ClashTuiStatusBar);
