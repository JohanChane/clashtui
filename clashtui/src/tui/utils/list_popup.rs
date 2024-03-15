use crate::tui::{tools, EventState, Visibility};
use ui::event::{Event, KeyEventKind};
use ratatui::{prelude as Ra, widgets as Raw};
use std::cmp::{max, min};
use ui::widgets::List;

use super::Keys;

pub struct PopUp(List);
impl PopUp {
    pub fn new(title: String) -> Self {
        let mut l = List::new(title);
        l.hide();
        Self(l)
    }
    pub fn set_items<I, T>(&mut self, items: I)
    where
        I: Iterator<Item = T>,
        T: Into<String>,
    {
        self.0.set_items(items.map(|v| v.into()).collect())
    }
}
impl Visibility for PopUp {
    fn is_visible(&self) -> bool {
        self.0.is_visible()
    }

    fn show(&mut self) {
        self.0.show()
    }

    fn hide(&mut self) {
        self.0.hide()
    }

    fn set_visible(&mut self, b: bool) {
        self.0.set_visible(b)
    }
}

impl PopUp {
    pub fn event(&mut self, ev: &Event) -> Result<EventState, ui::Infailable> {
        if !self.0.is_visible() {
            return Ok(EventState::NotConsumed);
        }

        if let Event::Key(key) = ev {
            if key.kind == KeyEventKind::Press {
                match key.code.into() {
                    Keys::Esc => self.0.hide(),

                    _ => return self.0.event(ev),
                };
            }
        }

        Ok(EventState::WorkDone)
    }
    pub fn draw(&mut self, f: &mut Ra::Frame, _area: Ra::Rect) {
        if !self.0.is_visible() {
            return;
        }
        // 自适应
        let items = self.0.get_items();
        let item_len = items.len();
        let max_item_width = items
            .iter()
            .map(|s| s.as_str())
            .map(Raw::ListItem::new)
            .map(|i| i.width())
            .max()
            .unwrap_or(0);
        let dialog_width = max(min(max_item_width + 2, f.size().width as usize - 4), 60); // min_width = 60
        let dialog_height = min(item_len + 2, f.size().height as usize - 6);
        let area = tools::centered_lenght_rect(dialog_width as u16, dialog_height as u16, f.size());

        f.render_widget(Raw::Clear, area);
        self.0.draw(f, area, true);
    }
}
