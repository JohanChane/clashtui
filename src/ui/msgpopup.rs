use anyhow::{bail, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::style::{Color, Modifier, Style};
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap},
};
use std::cmp::{max, min};
use std::rc::Rc;

use super::widgets::helper;
use super::EventState;
use crate::visible_methods;

pub struct MsgPopup {
    title: String,
    pub is_visible: bool,
    msg: Vec<String>,
    scroll_v: u16,
    scroll_h: u16,
}

impl MsgPopup {
    pub fn new() -> Self {
        Self {
            title: String::new(),
            is_visible: false,
            msg: vec![],
            scroll_v: 0,
            scroll_h: 0,
        }
    }

    pub fn event(&mut self, ev: &Event) -> Result<EventState> {
        if !self.is_visible {
            return Ok(EventState::NotConsumed);
        }

        if let Event::Key(key) = ev {
            if key.kind != KeyEventKind::Press {
                return Ok(EventState::NotConsumed);
            }
            match key.code {
                KeyCode::Esc => {
                    self.hide();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.scroll_down();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.scroll_up();
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    self.scroll_left();
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    self.scroll_right();
                }
                _ => {}
            }
        }

        Ok(EventState::WorkDone)
    }

    pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>) {
        if !self.is_visible {
            return;
        }

        let text: Vec<Line> = self
            .msg
            .iter()
            .map(|s| {
                Line::from(Span::styled(
                    s,
                    Style::default().fg(Color::Rgb(46, 204, 113)),
                ))
            })
            .collect();

        // 自适应
        let max_item_width = text.iter().map(|i| i.width()).max().unwrap_or(0);
        let dialog_width = max(min(max_item_width + 2, f.size().width as usize - 4), 60); // min_width = 60
        let dialog_height = min(text.len() + 2, f.size().height as usize - 6);
        let area =
            helper::centered_lenght_rect(dialog_width as u16, dialog_height as u16, f.size());

        let paragraph = if text.len() == 1 && max_item_width < area.width as usize {
            Paragraph::new(text)
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Center) // 会导致无法水平滚动
        } else {
            Paragraph::new(text).scroll((self.scroll_v, self.scroll_h))
        };

        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(0, 102, 102)));

        f.render_widget(Clear, area);
        f.render_widget(paragraph.clone().block(block), area);
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_v > 0 {
            self.scroll_v -= 1;
        }
    }
    pub fn scroll_down(&mut self) {
        self.scroll_v += 1;
    }
    pub fn scroll_left(&mut self) {
        if self.scroll_h > 0 {
            self.scroll_h -= 1;
        }
    }
    pub fn scroll_right(&mut self) {
        self.scroll_h += 1;
    }

    pub fn is_visible(&self) -> bool {
        self.is_visible
    }
    pub fn show(&mut self) {
        self.is_visible = true;
    }
    pub fn hide(&mut self) {
        self.is_visible = false;
        self.msg.clear();
        self.scroll_v = 0;
        self.scroll_h = 0;
    }
    pub fn set_msg(&mut self, msg: Vec<String>) {
        self.msg = msg;
    }

    fn get_msg(&self) -> &Vec<String> {
        &self.msg
    }

    pub fn clear_msg(&mut self) {
        self.msg.clear();
    }
    pub fn push_txt_msg(&mut self, msg: String) {
        self.msg.push(msg);
    }
    pub fn push_list_msg(&mut self, msg: Vec<String>) {
        self.msg.extend(msg);
    }
}
