use crate::tui::{tools, EventState, Theme, Visibility};
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{prelude as Ra, widgets as Raw};
use std::{
    cmp::{max, min},
    collections::HashMap,
};

#[derive(Visibility)]
pub struct InfoPopUp {
    title: String,
    is_visible: bool,
    items: HashMap<Infos, String>,
    list_state: Raw::ListState,
    scrollbar: Raw::ScrollbarState,
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
        }
    }
    pub fn with_items(mihomover: &str) -> Self {
        let mut items = HashMap::new();
        items.insert(Infos::TuiVer, crate::VERSION.to_string());
        items.insert(Infos::MihomoVer, mihomover.to_owned());
        let len = items.len();
        Self {
            title: "Info".to_string(),
            is_visible: false,
            items,
            list_state: Raw::ListState::default().with_selected(Some(0)),
            scrollbar: Raw::ScrollbarState::default().content_length(len),
        }
    }
}

impl InfoPopUp {
    pub fn event(&mut self, ev: &Event) -> Result<EventState, ui::Infailable> {
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
    pub fn draw(&mut self, f: &mut Ra::Frame, _area: Ra::Rect) {
        use Ra::Style;
        if !self.is_visible {
            return;
        }

        let items: Vec<Raw::ListItem> = [Infos::TuiVer, Infos::MihomoVer]
            .into_iter()
            .filter_map(|k| self.items.get_key_value(&k))
            .map(|i| Raw::ListItem::new(format!("{}{}", i.0, i.1)).style(Style::default()))
            .collect();

        // 自适应
        let item_len = items.len();
        let max_item_width = items.iter().map(|i| i.width()).max().unwrap_or(0);
        let dialog_width = max(min(max_item_width + 2, f.size().width as usize - 4), 60); // min_width = 60
        let dialog_height = min(item_len + 2, f.size().height as usize - 6);
        let area = tools::centered_lenght_rect(dialog_width as u16, dialog_height as u16, f.size());

        let list = Raw::List::new(items)
            .block(
                Raw::Block::default()
                    .borders(Raw::Borders::ALL)
                    .border_style(Style::default().fg(Theme::get().list_block_fouced_fg))
                    .title(self.title.clone()),
            )
            .highlight_style(
                Style::default()
                    .bg(Theme::get().list_hl_bg_fouced)
                    .add_modifier(Ra::Modifier::BOLD),
            );

        f.render_widget(Raw::Clear, area);
        f.render_stateful_widget(list, area, &mut self.list_state);
        if self.items.len() + 2 > area.height as usize {
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
    fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    self.scrollbar.first();
                    0
                } else {
                    self.scrollbar.next();
                    i + 1
                }
            }
            None => {
                self.scrollbar.first();
                0
            }
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
                    self.scrollbar.last();
                    self.items.len() - 1
                } else {
                    self.scrollbar.prev();
                    i - 1
                }
            }
            None => {
                self.scrollbar.last();
                0
            }
        };
        self.list_state.select(Some(i));
    }
}
