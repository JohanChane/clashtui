use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::prelude as Ra;
use ratatui::widgets as Raw;

use super::tools;
use super::PopMsg;
use crate::tui::{Drawable, EventState, Theme};

/// Pop a Message Window,
/// use arrow keys or `j\k\h\l`(vim-like) to navigate.
///
/// Can handle up to `4` choices.
///
/// - If press `Esc/n`, return [`EventState::Cancel`].
/// - If press `y`, return [`EventState::Yes`].
/// - If press `o`, return [`EventState::Choice2`].
/// - If press `t`, return [`EventState::Choice3`].
#[derive(Default)]
pub struct ConfirmPopup {
    msg: Option<PopMsg>,
    scroll_v: u16,
    scroll_h: u16,
}

impl Drawable for ConfirmPopup {
    /// No need to [clear](Raw::clear), or plan aera
    fn render(&mut self, f: &mut ratatui::Frame, _: ratatui::layout::Rect, _: bool) {
        let prompt = if let Some(PopMsg::Ask(_, ch2, ch3)) = self.msg.as_ref() {
            if let Some(ch2) = ch2 {
                if let Some(ch3) = ch3 {
                    format!("Press y for Yes, n for No, o for {ch2}, t for {ch3}")
                } else {
                    format!("Press y for Yes, n for No, o for {ch2}")
                }
            } else {
                "Press y for Yes, n for No".to_owned()
            }
        } else {
            "Press Esc to close".to_owned()
        };
        let text: Vec<Ra::Line> = if let Some(msg) = &self.msg {
            match msg {
                PopMsg::Ask(msg, ..) | PopMsg::Prompt(msg) => msg,
            }
            .iter()
            .chain([&prompt])
            .map(|s| {
                Ra::Line::from(Ra::Span::styled(
                    s,
                    Ra::Style::default().fg(Theme::get().popup_text_fg),
                ))
            })
            .collect()
        } else {
            return;
        };

        use std::cmp::{max, min};
        // 自适应
        let max_item_width = text.iter().map(|i| i.width()).max().unwrap_or(0);
        let area = {
            let dialog_width = max(min(max_item_width + 2, f.area().width as usize - 4), 60); // min_width = 60
            let dialog_height = min(
                if text.is_empty() { 3 } else { text.len() + 2 },
                f.area().height as usize - 6,
            );
            tools::centered_rect(
                Ra::Constraint::Length(dialog_width as u16),
                Ra::Constraint::Length(dialog_height as u16),
                f.area(),
            )
        };

        let paragraph = if text.len() == 1 && max_item_width < area.width as usize {
            Raw::Paragraph::new(text)
                .wrap(Raw::Wrap { trim: true })
                .alignment(Ra::Alignment::Center)
        } else {
            Raw::Paragraph::new(text).scroll((self.scroll_v, self.scroll_h))
        };

        let block = Raw::Block::new()
            .borders(Raw::Borders::ALL)
            .border_style(Ra::Style::default().fg(Theme::get().popup_block_fg))
            .title("Msg");

        f.render_widget(Raw::Clear, area);
        f.render_widget(paragraph.block(block), area);
    }

    fn handle_key_event(&mut self, ev: &crossterm::event::KeyEvent) -> EventState {
        if ev.kind != KeyEventKind::Press {
            return EventState::NotConsumed;
        }
        match ev.code {
            KeyCode::Down | KeyCode::Char('j') => self.scroll_down(),
            KeyCode::Up | KeyCode::Char('k') => self.scroll_up(),
            KeyCode::Left | KeyCode::Char('h') => self.scroll_left(),
            KeyCode::Right | KeyCode::Char('l') => self.scroll_right(),
            KeyCode::Char('n') | KeyCode::Esc => return EventState::Cancel,
            KeyCode::Char('y') => return EventState::Yes,
            KeyCode::Char('o') => return EventState::Choice2,
            KeyCode::Char('t') => return EventState::Choice3,
            _ => return EventState::NotConsumed,
        }
        EventState::WorkDone
    }
}

impl ConfirmPopup {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn show_msg(&mut self, msg: PopMsg) {
        self.msg.replace(msg);
    }
    pub fn clear(&mut self) {
        self.msg = None;
    }

    fn scroll_up(&mut self) {
        if self.scroll_v > 0 {
            self.scroll_v -= 1;
        }
    }
    fn scroll_down(&mut self) {
        self.scroll_v += 1;
    }
    fn scroll_left(&mut self) {
        if self.scroll_h > 0 {
            self.scroll_h -= 1;
        }
    }
    fn scroll_right(&mut self) {
        self.scroll_h += 1;
    }
}
