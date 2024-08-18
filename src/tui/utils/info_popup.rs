use super::list_popup::PopUp;
use crate::tui::{EventState, Visibility};
use crate::ui::event::Event;
use ratatui::prelude as Ra;
use std::collections::HashMap;

#[derive(Clone, PartialEq, Eq, Hash, Copy)]
pub enum Infos {
    TuiVer,
    MihomoVer,
    MihomoLoglevel,
    MihomoIpv6,
    MihomoAllowLan,
    MihomoGlobalUa,
}
impl core::fmt::Display for Infos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Infos::TuiVer => "ClashTui:",
                Infos::MihomoVer => "Mihomo:",
                Infos::MihomoLoglevel => todo!(),
                Infos::MihomoIpv6 => todo!(),
                Infos::MihomoAllowLan => todo!(),
                Infos::MihomoGlobalUa => todo!(),
            }
        )
    }
}

pub struct InfoPopUp {
    pub inner: PopUp,
    pub items: HashMap<Infos, String>,
}
impl InfoPopUp {
    #[allow(unused)]
    pub fn set_items(&mut self, item: Infos, cont: &str) {
        self.items.insert(item, cont.to_string());
        let mut strs: Vec<String> = self.items.iter().map(|(k, v)| format!("{k}:{v}")).collect();
        strs.sort(); // make sure A->Z
        self.inner.set_items(strs.iter())
    }
    pub fn new() -> Self {
        let mut items = HashMap::new();
        items.insert(Infos::TuiVer, crate::utils::consts::VERSION.to_string());
        let mut inner = PopUp::new("Info".to_string());
        inner.set_items(items.iter().map(|(k, v)| format!("{k}:{v}")));
        Self { inner, items }
    }
}

impl InfoPopUp {
    pub fn event(&mut self, ev: &Event) -> Result<EventState, crate::ui::Infailable> {
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
