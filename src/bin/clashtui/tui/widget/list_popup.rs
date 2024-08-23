use crossterm::event::KeyCode;
use crossterm::event::KeyEventKind;
use ratatui::prelude as Ra;
use ratatui::widgets as Raw;

use super::tools;
use crate::tui::misc::EventState;
use crate::tui::{Drawable, Theme};

#[derive(Default)]
pub struct ListPopup {
    title: String,
    items: Vec<String>,
    list_state: Raw::ListState,
    scrollbar: Raw::ScrollbarState,
}

impl ListPopup {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn set(&mut self, title: &str, items: Vec<String>) {
        self.title = title.to_owned();
        self.scrollbar = self.scrollbar.content_length(items.len());
        self.items = items;
    }
}

impl Drawable for ListPopup {
    /// No need to [Raw::clear], or plan aera
    fn render(&mut self, f: &mut ratatui::Frame, _: ratatui::layout::Rect, _: bool) {
        let area = tools::centered_percent_rect(60, 60, f.area());
        f.render_widget(Raw::Clear, area);
        let list =
            Raw::List::from_iter(self.items.iter().map(|i| {
                Raw::ListItem::new(Ra::Line::from(i.as_str())).style(Ra::Style::default())
            }));
        f.render_stateful_widget(
            list.block(
                Raw::Block::default()
                    .borders(Raw::Borders::ALL)
                    .border_style(Ra::Style::default().fg(Theme::get().list_block_fouced_fg))
                    .title(self.title.as_str()),
            )
            .highlight_style(
                Ra::Style::default()
                    .bg(Theme::get().list_hl_bg_fouced)
                    .add_modifier(Ra::Modifier::BOLD),
            ),
            area,
            &mut self.list_state,
        );

        if self.items.len() + 2 > area.height as usize {
            f.render_stateful_widget(
                Raw::Scrollbar::default()
                    .orientation(Raw::ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("↑"))
                    .end_symbol(Some("↓")),
                area,
                &mut self.scrollbar,
            );
        }
    }
    /// close this by [EventState::Cancel]
    fn handle_key_event(
        &mut self,
        ev: &crossterm::event::KeyEvent,
    ) -> crate::tui::misc::EventState {
        if ev.kind != KeyEventKind::Press {
            return EventState::NotConsumed;
        }
        match ev.code {
            KeyCode::Down | KeyCode::Char('j') => self.next(),
            KeyCode::Up | KeyCode::Char('k') => self.previous(),
            KeyCode::Esc => return EventState::Cancel,
            _ => return EventState::NotConsumed,
        };
        EventState::WorkDone
    }
}

impl ListPopup {
    fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    self.scrollbar.first();
                    0
                } else {
                    self.scrollbar.next();
                    i + 1
                }
            }
            None => {
                self.scrollbar.first();
                0
            }
        };
        self.list_state.select(Some(i));
    }

    fn previous(&mut self) {
        if self.items.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.scrollbar.last();
                    self.items.len() - 1
                } else {
                    self.scrollbar.prev();
                    i - 1
                }
            }
            None => {
                self.scrollbar.last();
                0
            }
        };
        self.list_state.select(Some(i));
    }
}
