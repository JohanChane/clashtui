use crate::tui::tab::TuiTab;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};
use std::rc::Rc;

use crate::tui::theme::Theme;
use super::chord::key_event_to_str;
use super::tab::KeyCombo;

#[derive(Default)]
pub struct HelpPanel {
    visible: bool,
}

impl HelpPanel {
    pub fn is_active(&self) -> bool {
        self.visible
    }

    pub fn toggle(&mut self) {
        self.visible ^= true;
    }

    pub fn dismiss(&mut self) {
        self.visible = false;
    }
}

const HELP_WIDTH: u16 = 60;

pub fn render_help(f: &mut ratatui::Frame, tab: &impl TuiTab) {
    let shortcuts = tab.shortcuts();
    let tab_title = tab.title();

    let global_shortcuts: Rc<[(KeyCombo, &str)]> = Rc::new([
        (KeyCombo(vec![]), "Switch tab 1-7"),
        (KeyCombo(vec![]), "Cycle tabs"),
        (KeyCombo(vec![]), "Toggle help"),
        (KeyCombo(vec![]), "Quit"),
        (KeyCombo(vec![]), "Quit"),
        (KeyCombo(vec![]), "Open app config dir"),
        (KeyCombo(vec![]), "Open clash config dir"),
        (KeyCombo(vec![]), "Start core service"),
        (KeyCombo(vec![]), "Close all connections"),
    ]);

    let global_labels: &[&str] = &["1-7", "<Tab>", "?", "q", "C-c", "C-g c", "C-g m", "C-g f", "C-g t"];

    let tab_entries = shortcuts.len();
    let global_entries = global_shortcuts.len();

    let tab_cols = if tab_entries > 4 { 2 } else { 1 };
    let global_cols = if global_entries > 4 { 2 } else { 1 };

    let tab_rows = ((tab_entries + tab_cols - 1) / tab_cols).max(1);
    let global_rows = ((global_entries + global_cols - 1) / global_cols).max(1);

    let total_height = 2 // borders
        + 1 // tab section header
        + tab_rows as u16
        + 1 // blank separator
        + 1 // global section header
        + global_rows as u16;

    let area = f.area();
    let popup_area = Rect {
        x: area
            .x
            .saturating_add(area.width.saturating_sub(HELP_WIDTH) / 2),
        y: area
            .y
            .saturating_add(area.height.saturating_sub(total_height) / 2),
        width: HELP_WIDTH.min(area.width),
        height: total_height.min(area.height),
    };

    f.render_widget(Clear, popup_area);

    let block = Block::bordered()
        .title(" Help ")
        .title_alignment(Alignment::Left);
    f.render_widget(block.clone(), popup_area);

    let inner = block.inner(popup_area);

    let tab_section_height = 1 + tab_rows as u16;
    let sections = Layout::vertical([
        Constraint::Length(tab_section_height + 1), // +1 for blank separator
        Constraint::Fill(1),
    ])
    .split(inner);

    render_shortcut_section(
        f,
        sections[0],
        &format!("Tab Shortcuts — {}", tab_title),
        shortcuts,
        tab_cols,
        tab_rows as u16,
        None,
    );

    let lower_inner_height = sections[1].height;
    if lower_inner_height > 0 {
        let lower_sections = Layout::vertical([
            Constraint::Length(1 + global_rows as u16),
            Constraint::Fill(1),
        ])
        .split(sections[1]);
        render_shortcut_section(
            f,
            lower_sections[0],
            "Global Shortcuts",
            &global_shortcuts,
            global_cols,
            global_rows as u16,
            Some(global_labels),
        );
    }
}

fn render_shortcut_section(
    f: &mut ratatui::Frame,
    area: Rect,
    title: &str,
    shortcuts: &[(KeyCombo, &str)],
    cols: usize,
    _rows: u16,
    custom_labels: Option<&[&str]>,
) {
    let header = Line::raw(title).bold();
    f.render_widget(Paragraph::new(header), Rect { height: 1, ..area });

    if shortcuts.is_empty() {
        return;
    }

    let accent = Theme::get().popup.text;

    let body_area = Rect {
        y: area.y + 1,
        height: area.height.saturating_sub(1),
        ..area
    };

    let col_widths: Vec<_> = (0..cols)
        .map(|_| Constraint::Ratio(1, cols as u32))
        .collect();
    let col_areas = Layout::horizontal(&col_widths).split(body_area);

    let items_per_col = (shortcuts.len() + cols - 1) / cols;

    for (col_idx, col_area) in col_areas.iter().enumerate().take(cols) {
        let lines: Vec<Line> = shortcuts
            .iter()
            .skip(col_idx * items_per_col)
            .take(items_per_col)
            .enumerate()
            .map(|(i, (combo, desc))| {
                let key_str: String = if let Some(labels) = custom_labels {
                    labels[col_idx * items_per_col + i].to_owned()
                } else {
                    combo
                        .iter()
                        .map(|k| key_event_to_str(k))
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                Line::from(vec![
                    Span::raw(" "),
                    Span::styled(key_str, Style::new().dim()),
                    Span::raw("  "),
                    Span::styled(*desc, accent),
                ])
            })
            .collect();

        f.render_widget(Paragraph::new(lines), *col_area);
    }
}
