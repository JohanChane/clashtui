use super::*;

impl FrontEnd {
    pub(super) fn render_tabbar(&self, f: &mut Frame, area: Rect) {
        let tab_titles: Vec<Ra::text::Line> = self
            .tabs
            .iter()
            .map(|tab| {
                Ra::text::Line::from(Ra::Span::styled(
                    tab.to_string(),
                    Theme::get().bars.tabbar_text,
                ))
            })
            .collect();
        let this = Raw::Tabs::new(tab_titles)
            .block(Raw::Block::default().borders(Raw::Borders::ALL))
            .highlight_style(Theme::get().bars.tabbar_highlight)
            .select(self.tab_index);
        f.render_widget(this, area);
    }

    pub(super) fn render_statusbar(&self, f: &mut Frame, area: Rect) {
        // load from local cache
        let this = Raw::Paragraph::new(Ra::Span::styled(
            &self.state,
            Theme::get().bars.statusbar_text,
        ))
        //.alignment(ratatui::prelude::Alignment::Right)
        .wrap(Raw::Wrap { trim: true })
        .block(Raw::Block::new().borders(Raw::Borders::ALL));
        f.render_widget(this, area);
    }
}
