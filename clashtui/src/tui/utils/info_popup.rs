use crate::tui::{EventState, Visibility};
use ratatui::prelude as Ra;
use std::collections::HashMap;
use ui::event::Event;

use super::list_popup::PopUp;

pub struct InfoPopUp {
    inner: PopUp,
    items: HashMap<Infos, String>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
enum Infos {
    TuiVer,
    MihomoVer,
}
impl core::fmt::Display for Infos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Infos::TuiVer => "ClashTui:".to_string(),
                Infos::MihomoVer => "Mihomo:".to_string(),
            }
        )
    }
}
impl InfoPopUp {
    #[allow(unused)]
    pub fn set_items(&mut self, mihomover: Option<&String>) {
        if let Some(v) = mihomover {
            self.items.insert(Infos::MihomoVer, v.clone());
        } else {
            return;
        }
        self.inner
            .set_items(self.items.iter().map(|(k, v)| format!("{k}:{v}")))
    }
    pub fn with_items(mihomover: &str) -> Self {
        let mut items = HashMap::new();
        items.insert(Infos::TuiVer, crate::utils::VERSION.to_string());
        items.insert(Infos::MihomoVer, mihomover.to_owned());
        let mut inner = PopUp::new("Info".to_string());
        inner.set_items(items.iter().map(|(k, v)| format!("{k}:{v}")));
        Self { items, inner }
    }
}

impl InfoPopUp {
    pub fn event(&mut self, ev: &Event) -> Result<EventState, ui::Infailable> {
        self.inner.event(ev)
    }
    pub fn draw(&mut self, f: &mut Ra::Frame, area: Ra::Rect) {
        self.inner.draw(f, area)
    }
}

impl Visibility for InfoPopUp {
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
