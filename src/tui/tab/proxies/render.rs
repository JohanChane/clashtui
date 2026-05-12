use super::super::dev::*;
use super::content::Proxies;
use super::tree::{NodeType, SortMode};
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

    // Compute filtered view
    let all_nodes = &content.tree.nodes;
    let filtered_indices: Vec<usize> = all_nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| {
            content.filter.as_deref().is_none_or(|pat| {
                node.name.to_lowercase().contains(&pat.to_lowercase())
            })
        })
        .map(|(i, _)| i)
        .collect();

    let current = state.selected().unwrap_or(0);
    let filter_cursor = if content.filter.is_some() && !filtered_indices.is_empty() {
        // Snap cursor to nearest visible match
        if filtered_indices.contains(&current) {
            filtered_indices.iter().position(|&i| i == current)
        } else {
            // Find nearest match (first index >= current, or last)
            filtered_indices
                .iter()
                .position(|&i| i >= current)
                .or_else(|| Some(filtered_indices.len().saturating_sub(1)))
        }
    } else {
        if current >= all_nodes.len() {
            None
        } else {
            Some(current)
        }
    };

    // Build footer
    let mut footer_parts: Vec<String> = Vec::new();

    // Sort indicator
    if current < all_nodes.len() {
        let node = &all_nodes[current];
        let group_resolved: Option<&str> = match node.node_type {
            NodeType::Folder => Some(node.name.as_str()),
            NodeType::Link | NodeType::File => node.parent.as_deref(),
        };
        if let Some(gname) = group_resolved {
            if let Some(idx) = content.tree.find_folder_index(gname) {
                match content.tree.nodes[idx].sort_mode {
                    SortMode::ByDelay => footer_parts.push("delay ".to_owned()),
                    SortMode::ByName => footer_parts.push("name ".to_owned()),
                    SortMode::None => {}
                }
            }
        }
    }

    // Filter indicator
    if let Some(ref f) = content.filter {
        footer_parts.push(format!("/ {f} "));
    } else {
        footer_parts.push("/: Filter ".to_owned());
    }

    let footer = footer_parts.join("");

    let block = if let Some(ref s) = spinner_str {
        block.title_bottom(Line::raw(s.as_str()))
    } else if !footer.is_empty() {
        block.title_bottom(Line::raw(footer).right_aligned().reversed())
    } else {
        block
    };

    let items: Vec<ListItem> = filtered_indices
        .iter()
        .map(|&i| {
            let node = &all_nodes[i];
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
                _ => ratatui::style::Style::default().fg(Color::Rgb(220, 220, 220)),
            };

            ListItem::new(Line::styled(line, style))
        })
        .collect();

    // Update state cursor for filtered view
    if content.filter.is_some() {
        if let Some(fc) = filter_cursor {
            if fc < items.len() {
                state.select(Some(fc));
            } else {
                state.select(None);
            }
        } else {
            state.select(None);
        }
    }

    let list = List::new(items)
        .block(block)
        .highlight_style(Theme::get().tab.item_highlighted);

    f.render_stateful_widget(list, area, state);
}
