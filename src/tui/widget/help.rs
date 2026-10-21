use crate::tui::key::EffectiveKeymap;
use crate::tui::key::{Key, KeyDescRef};
use crossterm::event::KeyCode;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use crate::tui::theme::Theme;

#[derive(Default)]
pub struct HelpPanel {
    visible: bool,
    scroll: usize,
    max_scroll: usize,
    page_height: usize,
}

impl HelpPanel {
    pub fn is_active(&self) -> bool {
        self.visible
    }

    pub fn toggle(&mut self) {
        self.visible ^= true;
        self.scroll = 0;
    }

    pub fn dismiss(&mut self) {
        self.visible = false;
        self.scroll = 0;
    }

    pub fn handle_key_event(&mut self, key: &Key) {
        if !key.modifiers.is_empty() {
            self.dismiss();
            return;
        }
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.scroll += 1,
            KeyCode::Up | KeyCode::Char('k') => self.scroll = self.scroll.saturating_sub(1),
            KeyCode::PageDown => self.scroll += self.page_height.max(1),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(self.page_height.max(1)),
            KeyCode::Home => self.scroll = 0,
            KeyCode::End => self.scroll = self.max_scroll,
            _ => self.dismiss(),
        }
        self.scroll = self.scroll.min(self.max_scroll);
    }

    pub fn render(&mut self, f: &mut ratatui::Frame, title: &str, bindings: &EffectiveKeymap) {
        self.render_content(f, title, bindings.page_docs(), bindings.global_docs());
    }

    fn render_content(
        &mut self,
        f: &mut ratatui::Frame,
        tab_title: &str,
        shortcuts: KeyDescRef,
        global_shortcuts: KeyDescRef,
    ) {
        let area = f.area();
        let width = area.width.min(HELP_WIDTH);
        let columns = |count| if count > 4 && width >= 70 { 2 } else { 1 };
        let tab_cols = columns(shortcuts.len());
        let global_cols = columns(global_shortcuts.len());
        let tab_rows = shortcuts.len().div_ceil(tab_cols).max(1);
        let global_rows = global_shortcuts.len().div_ceil(global_cols).max(1);
        let content_height = 1 // tab section header
            + tab_rows
            + 1 // blank separator
            + 1 // global section header
            + global_rows;
        let total_height = content_height + 2; // borders
        let height = total_height.min(usize::from(area.height)) as u16;
        let popup_area = Rect {
            x: area.x + (area.width - width) / 2,
            y: area.y + (area.height - height) / 2,
            width,
            height,
        };
        let mut block = Block::bordered()
            .title(" Help ")
            .title_alignment(Alignment::Left);
        let inner = block.inner(popup_area);
        self.page_height = usize::from(inner.height);
        self.max_scroll = content_height.saturating_sub(self.page_height);
        self.scroll = self.scroll.min(self.max_scroll);
        let footer = if self.max_scroll > 0 {
            format!(
                " ↑↓/PgUp/PgDn scroll | Esc close | {}/{} ",
                self.scroll + 1,
                self.max_scroll + 1
            )
        } else {
            " Esc close ".to_owned()
        };
        block = block.title_bottom(Line::raw(footer));
        f.render_widget(Clear, popup_area);
        f.render_widget(block, popup_area);

        render_shortcut_section(
            f,
            inner,
            &format!("Tab Shortcuts — {tab_title}"),
            shortcuts,
            tab_cols,
            -(self.scroll as isize),
        );
        let global_start = 1 + tab_rows + 1; // +1 for blank separator
        render_shortcut_section(
            f,
            inner,
            "Global Shortcuts",
            global_shortcuts,
            global_cols,
            global_start as isize - self.scroll as isize,
        );
    }
}

const HELP_WIDTH: u16 = 80;

fn render_shortcut_section(
    f: &mut ratatui::Frame,
    area: Rect,
    title: &str,
    shortcuts: KeyDescRef,
    cols: usize,
    first_row: isize,
) {
    if first_row >= 0 && first_row < area.height as isize {
        let header = Line::raw(title).bold();
        f.render_widget(
            Paragraph::new(header),
            Rect {
                y: area.y + first_row as u16,
                height: 1,
                ..area
            },
        );
    }

    if shortcuts.is_empty() {
        return;
    }

    let accent = Theme::get().popup.text;

    let items_per_col = shortcuts.len().div_ceil(cols);
    let body_start = first_row + 1;
    let first_visible = body_start.saturating_neg().max(0) as usize;
    let last_visible = items_per_col.min((area.height as isize - body_start).max(0) as usize);
    if first_visible >= last_visible {
        return;
    }
    let body_area = Rect {
        y: area.y + body_start.max(0) as u16,
        height: (last_visible - first_visible) as u16,
        ..area
    };

    let col_widths: Vec<_> = (0..cols)
        .map(|_| Constraint::Ratio(1, cols as u32))
        .collect();
    let col_areas = Layout::horizontal(&col_widths).split(body_area);

    for (col_idx, col_area) in col_areas.iter().enumerate().take(cols) {
        let lines: Vec<Line> = shortcuts
            .iter()
            .skip(col_idx * items_per_col + first_visible)
            .take(last_visible - first_visible)
            .map(|(key_str, desc)| {
                Line::from(vec![
                    Span::raw(" "),
                    Span::styled(key_str, Style::new().dim()),
                    Span::raw("  "),
                    Span::styled(desc.as_str(), accent),
                ])
            })
            .collect();

        f.render_widget(Paragraph::new(lines), *col_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::key::{KeyDesc, Keymaps, Scope};
    use ratatui::{Terminal, backend::TestBackend};

    fn defaults() -> (KeyDesc, KeyDesc) {
        let maps = Keymaps::from_config(serde_yml::Mapping::new(), "mihomo").unwrap();
        let bindings = maps.get(Scope::Proxies);
        (
            bindings.page_docs().to_vec(),
            bindings.global_docs().to_vec(),
        )
    }

    fn screen(
        panel: &mut HelpPanel,
        width: u16,
        height: u16,
        page: KeyDescRef,
        globals: KeyDescRef,
    ) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|f| panel.render_content(f, "Proxies", page, globals))
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
    fn default_help_can_reveal_every_binding_in_an_80_by_24_terminal() {
        let (page, globals) = defaults();
        let mut panel = HelpPanel::default();
        panel.toggle();
        let mut seen = screen(&mut panel, 80, 24, &page, &globals);
        assert!(panel.max_scroll > 0);
        assert!(seen.contains("scroll"));
        while panel.scroll < panel.max_scroll {
            panel.handle_key_event(&"<Down>".parse().unwrap());
            seen.push_str(&screen(&mut panel, 80, 24, &page, &globals));
        }
        for (key, desc) in page.iter().chain(&globals) {
            assert!(
                seen.contains(&format!(" {key}  {desc}")),
                "missing {key}: {desc}"
            );
        }
        assert!(panel.is_active());
        panel.handle_key_event(&"<Esc>".parse().unwrap());
        assert!(!panel.is_active());
        panel.toggle();
        assert_eq!(panel.scroll, 0);
    }

    #[test]
    fn narrow_help_scrolls_both_sections_and_clamps_after_resize() {
        let (page, globals) = defaults();
        let mut panel = HelpPanel::default();
        panel.toggle();
        let mut seen = screen(&mut panel, 40, 8, &page, &globals);
        while panel.scroll < panel.max_scroll {
            panel.handle_key_event(&"j".parse().unwrap());
            seen.push_str(&screen(&mut panel, 40, 8, &page, &globals));
        }
        for (key, desc) in page.iter().chain(&globals) {
            assert!(
                seen.contains(&format!(" {key}  {desc}")),
                "missing {key}: {desc}"
            );
        }
        panel.handle_key_event(&"<Home>".parse().unwrap());
        assert_eq!(panel.scroll, 0);
        panel.handle_key_event(&"<PageDown>".parse().unwrap());
        assert_eq!(panel.scroll, panel.page_height);
        panel.handle_key_event(&"<PageUp>".parse().unwrap());
        assert_eq!(panel.scroll, 0);
        panel.handle_key_event(&"<End>".parse().unwrap());
        assert_eq!(panel.scroll, panel.max_scroll);
        screen(&mut panel, 100, 100, &page, &globals);
        assert_eq!(panel.scroll, 0);
        assert_eq!(panel.max_scroll, 0);
        for (width, height) in [(0, 0), (1, 1), (2, 2), (3, 3), (10, 4)] {
            screen(&mut panel, width, height, &page, &globals);
        }
    }
}
