use super::*;

impl FrontEnd {
    pub(super) fn render_tabbar(&self, f: &mut Frame, area: Rect) {
        let tab_titles: Vec<Ra::text::Line> = self
            .tabs
            .iter()
            .map(|tab| {
                Ra::text::Line::from(Ra::Span::styled(
                    tab.to_string(),
                    Ra::Style::default().fg(Theme::get().tabbar_text_fg),
                ))
            })
            .collect();
        let this = Raw::Tabs::new(tab_titles)
            .block(Raw::Block::default().borders(Raw::Borders::ALL))
            .highlight_style(Ra::Style::default().fg(Theme::get().tabbar_hl_fg))
            .select(self.tab_index);
        f.render_widget(this, area);
    }

    pub(super) fn render_statusbar(&self, f: &mut Frame, area: Rect) {
        // load from local cache
        let state = self
            .state
            .as_deref()
            .unwrap_or("Waiting State Cache Update");
        let this = Raw::Paragraph::new(Ra::Span::styled(
            state,
            Ra::Style::default().fg(Theme::get().statusbar_text_fg),
        ))
        //.alignment(ratatui::prelude::Alignment::Right)
        .wrap(Raw::Wrap { trim: true })
        .block(Raw::Block::new().borders(Raw::Borders::ALL));
        f.render_widget(this, area);
    }
}
