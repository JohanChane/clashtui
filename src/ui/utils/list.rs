use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::style::{Color, Modifier, Style};
use ratatui::{prelude::*, widgets::*};

use super::SharedTheme;
use crate::ui::EventState;
use crate::{fouce_methods, visible_methods};

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

pub struct ClashTuiList {
    title: String,
    is_visible: bool,
    is_fouce: bool,
    items: Vec<String>,
    list_state: ListState,
    scrollbar: ScrollbarState,

    theme: SharedTheme,
}

impl ClashTuiList {
    pub fn new(title: String, theme: SharedTheme) -> Self {
        Self {
            title,
            is_visible: true,
            is_fouce: true,
            items: vec![],
            list_state: ListState::default(),
            scrollbar: ScrollbarState::default(),

            theme,
        }
    }

    pub fn event(&mut self, ev: &Event) -> Result<EventState> {
        if !self.is_visible || !self.is_fouce {
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

    pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        if !self.is_visible {
            return;
        }

        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|i| {
                let lines = vec![Line::from(i.clone())];
                ListItem::new(lines).style(Style::default())
            })
            .collect();

        let item_len = items.len();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(if self.is_fouce {
                        self.theme.list_block_fg_fouced
                    } else {
                        self.theme.list_block_fg_unfouced
                    }))
                    .title(self.title.clone()),
            )
            .highlight_style(
                Style::default()
                    .bg(if self.is_fouce {
                        self.theme.list_hl_bg_fouced
                    } else {
                        Color::default()
                    })
                    .add_modifier(Modifier::BOLD),
            );

        f.render_stateful_widget(list, area, &mut self.list_state);

        if item_len > area.height as usize {
            self.scrollbar = self.scrollbar.content_length(item_len as u16);
            f.render_stateful_widget(
                Scrollbar::default()
                    .orientation(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("↑"))
                    .end_symbol(Some("↓")),
                area,
                &mut self.scrollbar,
            );
        }
    }

    pub fn selected(&self) -> Option<&String> {
        if self.items.len() == 0 {
            return None;
        }

        match self.list_state.selected() {
            Some(i) => Some(&self.items[i]),
            None => None,
        }
    }

    fn next(&mut self) {
        if self.items.len() == 0 {
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
        if self.items.len() == 0 {
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

        if self.list_state.selected() == None && self.items.len() > 0 {
            self.list_state.select(Some(0));
        }
    }

    pub fn get_items(&self) -> &Vec<String> {
        &self.items
    }

    pub fn select(&mut self, profile_name: &str) {
        if let Some(index) = self.items.iter().position(|item| item == profile_name) {
            self.list_state.select(Some(index));
        }
    }
}

visible_methods!(ClashTuiList);
fouce_methods!(ClashTuiList);
