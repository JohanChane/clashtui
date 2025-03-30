use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::{prelude as Ra, widgets as Raw};

use crate::tui::{Drawable, EventState, Theme};

/// Interactive list, mainly used as basic interface
///
/// Using arrow keys or j\k(vim-like) to navigate.
///
/// Support 'filter' to filter out display (items are unchanged)
pub struct List {
    title: String,
    filter: Option<String>,
    items: Vec<String>,
    extra: Option<Vec<String>>,
    state: Raw::ListState,
    scrollbar: Raw::ScrollbarState,
}
impl Drawable for List {
    fn render(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect, is_fouced: bool) {
        let list = if let Some(extras) = self.extra.as_ref() {
            Raw::List::from_iter(
                self.items
                    .iter()
                    .zip(extras.iter())
                    // filter content now
                    .filter_map(|(value, extra)| {
                        self.filter
                            .as_ref()
                            .is_none_or(|pat| value.contains(pat))
                            .then_some((value, extra))
                    })
                    .map(|(value, extra)| {
                        Raw::ListItem::new(Ra::Line::from(vec![
                            Ra::Span::raw(value),
                            Ra::Span::raw("("),
                            Ra::Span::raw(extra).style(Theme::get().profile_tab.update_interval),
                            Ra::Span::raw(")"),
                        ]))
                    }),
            )
        } else {
            Raw::List::from_iter(
                self.items
                    .iter()
                    .map_while(|value| {
                        self.filter
                            .as_ref()
                            .is_none_or(|pat| value.contains(pat))
                            .then_some(value)
                    })
                    .map(|i| Raw::ListItem::new(Ra::Line::from(i.as_str()))),
            )
        };
        f.render_stateful_widget(
            list.block(
                Raw::Block::default()
                    .borders(Raw::Borders::ALL)
                    .border_style(if is_fouced {
                        Theme::get().list.block_selected
                    } else {
                        Theme::get().list.block_unselected
                    })
                    .title(self.title.as_str()),
            )
            .highlight_style(if is_fouced {
                Theme::get().list.highlight
            } else {
                Theme::get().list.unhighlight
            }),
            area,
            &mut self.state,
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
    /// - Up/Down/j/k -> [EventState::WorkDone]
    /// - Enter -> [EventState::Yes]
    /// - Esc -> [EventState::Cancel]
    /// - unrecognized event -> [EventState::NotConsumed]
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
            filter: None,
            items: vec![],
            extra: None,
            state: Raw::ListState::default(),
            scrollbar: Raw::ScrollbarState::default(),
        }
    }

    /// Index of the selected item
    ///
    /// Returns `None` if no item is selected
    pub fn selected(&self) -> Option<usize> {
        self.state.selected()
    }

    fn next(&mut self) {
        self.scrollbar.next();
        self.state.select_next();
    }

    fn previous(&mut self) {
        if self.state.selected().is_none() {
            self.scrollbar.last();
        } else {
            self.scrollbar.prev();
        }
        self.state.select_previous();
    }

    pub fn set_items(&mut self, items: Vec<String>) {
        // list state will be correct automatically at render
        self.items = items;
        self.scrollbar = self.scrollbar.content_length(self.items.len());
    }

    pub fn set_extras<I>(&mut self, extra: I)
    where
        I: Iterator<Item = String> + ExactSizeIterator,
    {
        debug_assert_eq!(self.items.len(), extra.len());
        self.extra.replace(Vec::from_iter(extra));
    }

    pub fn get_items(&self) -> &Vec<String> {
        &self.items
    }
    pub fn get_items_mut(&mut self) -> &mut Vec<String> {
        &mut self.items
    }

    pub fn set_filter(&mut self, filter: String) {
        self.filter = filter.into();
    }
}
