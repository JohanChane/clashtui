use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{prelude as Ra, widgets as Raw};
use std::cmp::{max, min};

use crate::{utils::tools, EventState, Infailable, Theme};

/// Pop a Message Window
///
/// Using arrow keys or `j\k\h\l`(vim-like) to navigate.
/// Press Esc to close, do nothing for others
///
/// Not impl [Visibility][crate::Visibility] but impl the functions
#[derive(Default)]
pub struct MsgPopup {
    is_visible: bool,
    msg: Vec<String>,
    scroll_v: u16,
    scroll_h: u16,
}
impl MsgPopup {
    pub fn event(&mut self, ev: &Event) -> Result<EventState, Infailable> {
        if !self.is_visible {
            return Ok(EventState::NotConsumed);
        }

        if let Event::Key(key) = ev {
            if key.kind != KeyEventKind::Press {
                return Ok(EventState::NotConsumed);
            }
            match key.code {
                KeyCode::Esc => self.hide(),
                KeyCode::Down | KeyCode::Char('j') => self.scroll_down(),
                KeyCode::Up | KeyCode::Char('k') => self.scroll_up(),
                KeyCode::Left | KeyCode::Char('h') => self.scroll_left(),
                KeyCode::Right | KeyCode::Char('l') => self.scroll_right(),
                _ => {}
            }
        }

        Ok(EventState::WorkDone)
    }

    pub fn draw(&mut self, f: &mut Ra::Frame, _area: Ra::Rect) {
        //! area is only used to keep the args
        if !self.is_visible {
            return;
        }

        let text: Vec<Ra::Line> = self
            .msg
            .iter()
            .map(|s| {
                Ra::Line::from(Ra::Span::styled(
                    s,
                    Ra::Style::default().fg(Theme::get().popup_text_fg),
                ))
            })
            .collect();

        // 自适应
        let max_item_width = text.iter().map(|i| i.width()).max().unwrap_or(0);
        let dialog_width = max(min(max_item_width + 2, f.size().width as usize - 4), 60); // min_width = 60
        let dialog_height = min(if text.len() == 0 {3} else {text.len() + 2}, f.size().height as usize - 6);
        let area = tools::centered_lenght_rect(dialog_width as u16, dialog_height as u16, f.size());

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

    pub fn clear_msg(&mut self) {
        self.msg.clear();
    }
    pub fn push_txt_msg(&mut self, msg: String) {
        //self.msg.clear();     // let hide() clear msg.
        self.msg.push(msg);
    }
    pub fn push_list_msg(&mut self, msg: impl IntoIterator<Item = String>) {
        //self.msg.clear();     // let hide() clear msg.
        self.msg.extend(msg);
    }
}
