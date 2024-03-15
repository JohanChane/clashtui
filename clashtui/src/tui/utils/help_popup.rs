use super::list_popup::PopUp;
use crate::tui::{symbols::HELP, EventState, Visibility};
use ui::event::Event;
use ratatui::prelude as Ra;

pub struct HelpPopUp {
    inner: PopUp,
}

impl HelpPopUp {
    pub fn new() -> Self {
        let mut inner = PopUp::new("Help".to_string());
        inner.set_items(HELP.lines().map(|line| line.trim().to_string()));
        Self { inner }
    }
    pub fn event(&mut self, ev: &Event) -> Result<EventState, ui::Infailable> {
        self.inner.event(ev)
    }
    pub fn draw(&mut self, f: &mut Ra::Frame, area: Ra::Rect) {
        self.inner.draw(f, area)
    }
}

impl Visibility for HelpPopUp {
    fn is_visible(&self) -> bool {
        self.inner.is_visible()
    }

    fn show(&mut self) {
        self.inner.show()
    }

    fn hide(&mut self) {
        self.inner.hide()
    }

    fn set_visible(&mut self, b: bool) {
        self.inner.set_visible(b)
    }
}
