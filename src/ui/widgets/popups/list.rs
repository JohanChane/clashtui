use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::style::{Modifier, Style};
use ratatui::{prelude::*, widgets::*};
use std::cmp::{max, min};

use crate::ui::widgets::SharedTheme;

pub struct ClashTuiListPopup {
    title: String,
    is_visible: bool,
    items: Vec<String>,
    list_state: ListState,
    theme: SharedTheme,
}

use super::super::helper;
use crate::ui::EventState;
use crate::visible_methods;

impl ClashTuiListPopup {
    pub fn new(title: String, theme: SharedTheme) -> Self {
        Self {
            title,
            is_visible: false,
            items: vec![],
            list_state: ListState::default(),
            theme,
        }
    }

    pub fn event(&mut self, ev: &Event) -> Result<EventState> {
        if !self.is_visible {
            return Ok(EventState::NotConsumed);
        }

        if let Event::Key(key) = ev {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.next();
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.previous();
                    }
                    KeyCode::Esc => {
                        self.hide();
                    }
                    KeyCode::Enter => {
                        self.hide();
                    }
                    _ => {}
                };
            }
        }

        Ok(EventState::WorkDone)
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

        // Window size adaptation
        let item_len = items.len();
        let max_item_width = items.iter().map(|i| i.width()).max().unwrap_or(0);
        let dialog_width = max(min(max_item_width + 2, f.size().width as usize - 4), 60); // min_width = 60
        let dialog_height = min(item_len + 2, f.size().height as usize - 6);
        let area =
            helper::centered_lenght_rect(dialog_width as u16, dialog_height as u16, f.size());

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.theme.list_block_fg_fouced))
                    .title(self.title.clone()),
            )
            .highlight_style(
                Style::default()
                    .bg(self.theme.list_hl_bg_fouced)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_widget(Clear, area);
        f.render_stateful_widget(list, area, &mut self.list_state);
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
}

visible_methods!(ClashTuiListPopup);
