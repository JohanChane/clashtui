use super::super::dev::*;
use super::content::Proxies;
use super::tree::NodeType;
use crate::tui::theme::Theme;
use ratatui::text::Line;
use ratatui::widgets::{Block, List, ListItem};

pub fn render(content: &Proxies, f: &mut Frame, area: Rect, state: &mut ListState) {
    // Clamp cursor to valid range
    if let Some(idx) = state.selected() {
        let len = content.tree.len();
        if len == 0 {
            state.select(None);
        } else if idx >= len {
            state.select(Some(len.saturating_sub(1)));
        }
    } else if content.tree.len() > 0 {
        state.select(Some(0));
    }

    let block = Block::bordered()
        .border_style(Theme::get().tab.tab_focused)
        .title(Proxies::TITLE);

    let block = if content.tree.sort_by_delay {
        block.title_bottom(Line::raw(" delay ").right_aligned().reversed())
    } else if content.tree.sorted {
        block.title_bottom(Line::raw(" name ").right_aligned().reversed())
    } else {
        block
    };

    let spinner_str = content.testing_since.map(|since| {
        let elapsed = since.elapsed().as_millis() as usize;
        let spinner = ['|', '/', '-', '\\'];
        let c = spinner[(elapsed / 100) % 4];
        let msg = content.error.as_deref().unwrap_or("Testing...");
        format!(" {c} {msg}")
    });

    if content.tree.is_empty() {
        let msg = spinner_str.as_deref().unwrap_or(content.error.as_deref().unwrap_or(""));
        let widget = ratatui::widgets::Paragraph::new(msg).block(block);
        f.render_widget(widget, area);
        return;
    }

    let block = if let Some(ref s) = spinner_str {
        block.title_bottom(Line::raw(s.as_str()))
    } else {
        block
    };

    let items: Vec<ListItem> = content
        .tree
        .nodes
        .iter()
        .map(|node| {
            let indent = "  ".repeat(node.depth);
            let prefix = match node.node_type {
                NodeType::Folder => {
                    if node.expanded { "▼" } else { "▶" }
                }
                NodeType::Link => {
                    if node.is_now { "*" } else { " " }
                }
                NodeType::File => {
                    if node.is_now { "*" } else { " " }
                }
            };
            let type_str = if node.proxy_type.is_empty() {
                String::new()
            } else {
                format!("[{}]", node.proxy_type)
            };
            let delay_str = node.delay.map(|d| {
                if d == 0 {
                    "FAIL".to_owned()
                } else {
                    format!("{}ms", d)
                }
            }).unwrap_or_default();

            let line = format!(
                "{indent} {prefix} {}  {}  {}",
                node.name, type_str, delay_str,
            );

            let style = match node.node_type {
                NodeType::Folder => Theme::get().tab.tab_focused,
                NodeType::Link => ratatui::style::Style::default().fg(Color::Rgb(100, 180, 150)),
                _ => ratatui::style::Style::default(),
            };

            ListItem::new(Line::styled(line, style))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Theme::get().tab.item_highlighted);

    f.render_stateful_widget(list, area, state);
}
