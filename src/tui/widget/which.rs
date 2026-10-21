use crate::tui::{key::WhichHints, theme::Theme};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
};

/// Choices come from the same immutable context map used by App's matcher.
pub fn render(f: &mut Frame, hints: &WhichHints) {
    render_panel(f, f.area(), hints, Theme::get().popup.text);
}

fn render_panel(f: &mut Frame, area: Rect, hints: &WhichHints, accent: Style) {
    if hints.keys.is_empty() || area.width < 3 || area.height < 3 {
        return;
    }

    let cols = if hints.keys.len() > 4 && area.width >= 70 {
        2
    } else {
        1
    };
    let rows = hints
        .keys
        .len()
        .div_ceil(cols)
        .min(usize::from(area.height - 2));
    let height = rows as u16 + 2;
    let width = area.width.min(if cols == 1 { 40 } else { 70 });
    let bottom_gap = area.height.saturating_sub(height).min(2);
    let popup = Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + area.height - height - bottom_gap,
        width,
        height,
    };
    let hidden = hints.keys.len().saturating_sub(rows * cols);
    let footer = if hidden == 0 {
        " Esc cancel ".to_owned()
    } else {
        format!(" Esc cancel | +{hidden} more ")
    };
    let block = Block::bordered()
        .title(format!(" Which? {} ", hints.prefix))
        .title_bottom(Line::raw(footer).right_aligned());
    let inner = block.inner(popup);
    f.render_widget(Clear, popup);
    f.render_widget(block, popup);

    let columns = Layout::horizontal(vec![Constraint::Ratio(1, cols as u32); cols]).split(inner);
    for (column, area) in columns.iter().enumerate() {
        let lines: Vec<_> = hints
            .keys
            .iter()
            .skip(column * rows)
            .take(rows)
            .map(|(key, desc)| {
                Line::from(vec![
                    Span::raw(" "),
                    Span::styled(key.as_str(), accent),
                    Span::raw("  "),
                    Span::styled(desc.as_str(), Style::new().dim()),
                ])
            })
            .collect();
        f.render_widget(Paragraph::new(lines), *area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    fn screen(width: u16, height: u16, area: Option<Rect>, count: usize) -> String {
        let hints = WhichHints {
            prefix: "g".parse().unwrap(),
            name: "Nav".into(),
            keys: (0..count)
                .map(|i| (i.to_string(), format!("Custom action {i}")))
                .collect(),
        };
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|f| render_panel(f, area.unwrap_or(f.area()), &hints, Style::default()))
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn shows_prefix_remaining_keys_and_custom_descriptions() {
        let text = screen(80, 24, None, 2);
        assert!(text.contains("Which? g"));
        assert!(text.contains("0  Custom action 0"));
        assert!(text.contains("1  Custom action 1"));
        assert!(text.contains("Esc cancel"));
    }

    #[test]
    fn two_columns_show_every_candidate() {
        let text = screen(80, 24, None, 7);
        for i in 0..7 {
            assert!(text.contains(&format!("Custom action {i}")));
        }
    }

    #[test]
    fn adapts_to_narrow_small_and_offset_areas() {
        let text = screen(40, 12, None, 7);
        for i in 0..7 {
            assert!(text.contains(&format!("Custom action {i}")));
        }
        let text = screen(40, 5, None, 7);
        assert!(text.contains("+4 more"));
        let text = screen(100, 30, Some(Rect::new(10, 5, 40, 10)), 2);
        assert!(text.contains("Custom action 1"));
        for (width, height) in [(0, 0), (1, 1), (2, 2), (3, 3), (10, 4)] {
            screen(width, height, None, 7);
        }
        assert!(!screen(80, 24, None, 0).contains("Which?"));
    }
}
