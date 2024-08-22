use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::{prelude as Ra, widgets as Raw};

use crate::tui::{Drawable, EventState, Theme};

/// Interactive list, mainly used as basic interface
///
/// Using arrow keys or j\k(vim-like) to navigate.
pub struct List {
    title: String,
    items: Vec<String>,
    extra: Option<Vec<String>>,
    list_state: Raw::ListState,
    scrollbar: Raw::ScrollbarState,
}
impl Drawable for List {
    fn render(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect, is_fouced: bool) {
        let list = if let Some(extras) = self.extra.as_ref() {
            Raw::List::from_iter(self.items.iter().zip(extras.iter()).map(|(value, extra)| {
                Raw::ListItem::new(Ra::Line::from(vec![
                    Ra::Span::styled(value.to_owned(), Ra::Style::default()),
                    Ra::Span::styled("(".to_owned(), Ra::Style::default()),
                    Ra::Span::styled(
                        extra,
                        Ra::Style::default().fg(Theme::get().profile_update_interval_fg),
                    ),
                    Ra::Span::styled(")".to_owned(), Ra::Style::default()),
                ]))
            }))
        } else {
            Raw::List::from_iter(self.items.iter().map(|i| {
                Raw::ListItem::new(Ra::Line::from(i.as_str())).style(Ra::Style::default())
            }))
        };
        f.render_stateful_widget(
            list.block(
                Raw::Block::default()
                    .borders(Raw::Borders::ALL)
                    .border_style(Ra::Style::default().fg(if is_fouced {
                        Theme::get().list_block_fouced_fg
                    } else {
                        Theme::get().list_block_unfouced_fg
                    }))
                    .title(self.title.as_str()),
            )
            .highlight_style(
                Ra::Style::default()
                    .bg(if is_fouced {
                        Theme::get().list_hl_bg_fouced
                    } else {
                        Ra::Color::default()
                    })
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
    /// capture up/down/j/k and `Enter`/`Esc`, with `Enter` return [`EventState::Yes`], `Esc` return [`EventState::Cancel`]
    fn handle_key_event(&mut self, ev: &crossterm::event::KeyEvent) -> EventState {
        if ev.kind != KeyEventKind::Press {
            return EventState::NotConsumed;
        }
        match ev.code {
            KeyCode::Down | KeyCode::Char('j') => self.next(),
            KeyCode::Up | KeyCode::Char('k') => self.previous(),
            KeyCode::Enter => return EventState::Yes,
            KeyCode::Esc => return EventState::Cancel,
            _ => return EventState::NotConsumed,
        };
        EventState::WorkDone
    }
}

impl List {
    pub fn new(title: String) -> Self {
        Self {
            title,
            items: vec![],
            extra: None,
            list_state: Raw::ListState::default(),
            scrollbar: Raw::ScrollbarState::default(),
        }
    }

    /// Index of the selected item
    ///
    /// Returns `None` if no item is selected
    pub fn selected(&self) -> Option<usize> {
        if self.items.is_empty() {
            return None;
        }
        self.list_state.selected()
    }

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

    pub fn set_items(&mut self, items: Vec<String>) {
        match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.list_state.select(None);
                } else if i >= items.len() {
                    self.list_state.select(Some(items.len() - 1));
                }
            }
            None => self.list_state.select(None),
        }
        self.items = items;
        self.scrollbar = self.scrollbar.content_length(self.items.len());

        if self.list_state.selected().is_none() && !self.items.is_empty() {
            self.list_state.select(Some(0));
            self.scrollbar.first();
        }
    }

    pub fn set_extras<I>(&mut self, extra: I)
    where
        I: Iterator<Item = String> + ExactSizeIterator,
    {
        assert_eq!(self.items.len(), extra.len());
        self.extra.replace(Vec::from_iter(extra));
    }

    pub fn get_items(&self) -> &Vec<String> {
        &self.items
    }

    pub fn select(&mut self, name: &str) {
        if let Some(index) = self.items.iter().position(|item| item == name) {
            self.list_state.select(Some(index));
        }
    }
}
