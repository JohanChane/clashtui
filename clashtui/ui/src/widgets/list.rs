use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{prelude as Ra, widgets as Raw};

use crate::{utils::Theme, EventState, Infailable, Visibility};

/// Interactive list, mainly used as basic interface
///
/// Using arrow keys or j\k(vim-like) to navigate.
#[derive(Visibility)]
pub struct List {
    title: String,
    is_visible: bool,
    items: Vec<String>,
    extra: Option<Vec<String>>,
    list_state: Raw::ListState,
    scrollbar: Raw::ScrollbarState,
}

impl List {
    pub fn new(title: String) -> Self {
        Self {
            title,
            is_visible: true,
            items: vec![],
            extra: None,
            list_state: Raw::ListState::default(),
            scrollbar: Raw::ScrollbarState::default(),
        }
    }

    pub fn event(&mut self, ev: &Event) -> Result<EventState, Infailable> {
        if !self.is_visible {
            return Ok(EventState::NotConsumed);
        }

        if let Event::Key(key) = ev {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Down | KeyCode::Char('j') => self.next(),
                    KeyCode::Up | KeyCode::Char('k') => self.previous(),
                    _ => return Ok(EventState::NotConsumed),
                };
                return Ok(EventState::WorkDone);
            }
        }

        Ok(EventState::NotConsumed)
    }

    pub fn draw(&mut self, f: &mut Ra::Frame, area: Ra::Rect, is_fouced: bool) {
        if !self.is_visible {
            return;
        }

        f.render_stateful_widget(
            if let Some(vc) = self.extra.as_ref() {
                Raw::List::from_iter(self.items.iter().zip(vc.iter()).map(|(v, e)| {
                    Raw::ListItem::new(
                        Ra::Line::from(vec![
                             Ra::Span::styled(v.to_owned(), Ra::Style::default()),
                             Ra::Span::styled(" ".to_owned(), Ra::Style::default()),
                             //Ra::Span::styled(e, Ra::Style::default().fg(Ra::Color::Rgb(192, 192, 192)))
                             Ra::Span::styled(e, Ra::Style::default().fg(Theme::get().profile_update_interval_fg))
                        ])
                    )
                }))
            } else {
                Raw::List::from_iter(self.items.iter().map(|i| {
                    Raw::ListItem::new(Ra::Line::from(i.as_str())).style(Ra::Style::default())
                }))
            }
            .block(
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

    pub fn selected(&self) -> Option<&String> {
        if self.items.is_empty() {
            return None;
        }

        self.list_state.selected().map(|i| &self.items[i])
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
