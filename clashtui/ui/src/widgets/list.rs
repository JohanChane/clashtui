use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{prelude as Ra, widgets as Raw};

use crate::{utils::SharedTheme, EventState, Infallable, Visibility};

// struct ClashTuiScrollBar {
//     pub state: ScrollbarState,
//     pub pos: usize,
// }
//
// impl ClashTuiScrollBar {
//     pub fn new(pos: usize) -> Self {
//         Self {
//             state: ScrollbarState::default(),
//             pos,
//         }
//     }
//     pub fn next(&mut self) {
//         self.pos = self.pos.saturating_add(1);
//         self.state = self.state.position(self.pos as u16)
//     }
//
//     pub fn previous(&mut self) {
//         self.pos = self.pos.saturating_sub(1);
//         self.state = self.state.position(self.pos as u16)
//     }
// }

#[derive(Visibility)]
pub struct List {
    title: String,
    is_visible: bool,
    items: Vec<String>,
    list_state: Raw::ListState,
    scrollbar: Raw::ScrollbarState,

    theme: SharedTheme,
}

impl List {
    pub fn new(title: String, theme: SharedTheme) -> Self {
        Self {
            title,
            is_visible: true,
            items: vec![],
            list_state: Raw::ListState::default(),
            scrollbar: Raw::ScrollbarState::default(),

            theme,
        }
    }

    pub fn event(&mut self, ev: &Event) -> Result<EventState, Infallable> {
        if !self.is_visible {
            return Ok(EventState::NotConsumed);
        }

        let mut event_state = EventState::NotConsumed;
        if let Event::Key(key) = ev {
            if key.kind == KeyEventKind::Press {
                event_state = match key.code {
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.next();
                        EventState::WorkDone
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.previous();
                        EventState::WorkDone
                    }
                    _ => EventState::NotConsumed,
                };
            }
        }

        Ok(event_state)
    }

    pub fn draw(&mut self, f: &mut Ra::Frame, area: Ra::Rect, is_fouced: bool) {
        if !self.is_visible {
            return;
        }

        let items: Vec<Raw::ListItem> = self
            .items
            .iter()
            .map(|i| {
                let lines = vec![Ra::Line::from(i.clone())];
                Raw::ListItem::new(lines).style(Ra::Style::default())
            })
            .collect();

        let item_len = items.len();

        let list = Raw::List::new(items)
            .block(
                Raw::Block::default()
                    .borders(Raw::Borders::ALL)
                    .border_style(Ra::Style::default().fg(if is_fouced {
                        self.theme.list_block_fg_fouced
                    } else {
                        self.theme.list_block_fg_unfouced
                    }))
                    .title(self.title.clone()),
            )
            .highlight_style(
                Ra::Style::default()
                    .bg(if is_fouced {
                        self.theme.list_hl_bg_fouced
                    } else {
                        Ra::Color::default()
                    })
                    .add_modifier(Ra::Modifier::BOLD),
            );

        f.render_stateful_widget(list, area, &mut self.list_state);

        if item_len > area.height as usize {
            self.scrollbar = self.scrollbar.content_length(item_len);
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

        match self.list_state.selected() {
            Some(i) => Some(&self.items[i]),
            None => None,
        }
    }

    fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
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
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
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

        if self.list_state.selected().is_none() && !self.items.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    pub fn select(&mut self, name: &str) {
        if let Some(index) = self.items.iter().position(|item| item == name) {
            self.list_state.select(Some(index));
        }
    }
}
