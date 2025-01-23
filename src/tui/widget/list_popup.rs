use crossterm::event::KeyCode;
use crossterm::event::KeyEventKind;
use ratatui::prelude as Ra;
use ratatui::widgets as Raw;

use super::tools;
use super::PopMsg;
use crate::tui::misc::EventState;
use crate::tui::{Drawable, Theme};

#[derive(Default)]
pub struct ListPopup {
    title: String,
    items: Vec<String>,
    list_state: Raw::ListState,
    scrollbar: Raw::ScrollbarState,
    offset: usize,
}

impl ListPopup {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn set(&mut self, title: &str, items: Vec<String>) {
        self.list_state = Default::default();
        self.offset = 0;
        self.title = title.to_owned();
        self.scrollbar = Raw::ScrollbarState::new(items.len());
        self.items = items;
    }
    pub fn show_msg(&mut self, msg: PopMsg) {
        let prompt = if let PopMsg::Ask(_, chs) = &msg {
            match chs.len() {
                0 => "Press y for Yes, n for No".to_owned(),
                1 => format!("Press y for Yes, n for No, o for {}", chs[0]),
                2 => format!(
                    "Press y for Yes, n for No, o for {}, t for {}",
                    chs[0], chs[1]
                ),
                _ => unimplemented!("more than 2 extra choices!"),
            }
        } else {
            "Press Esc to close".to_owned()
        };
        match msg {
            PopMsg::Ask(vec, _) | PopMsg::Prompt(vec) => self.set("Msg", vec),
        }
        self.items.push(prompt);
    }
}

impl Drawable for ListPopup {
    /// No need to [Raw::clear], or plan aera
    fn render(&mut self, f: &mut ratatui::Frame, _: ratatui::layout::Rect, _: bool) {
        let area = {
            use std::cmp::{max, min};
            let max_item_width = self.items.iter().map(|i| i.len()).max().unwrap_or(0);
            let dialog_width = max(min(max_item_width + 2, f.area().width as usize - 4), 60); // min_width = 60
            let dialog_height = min(
                if self.items.is_empty() {
                    3
                } else {
                    self.items.len() + 2
                },
                f.area().height as usize - 6,
            );
            tools::centered_rect(
                Ra::Constraint::Length(dialog_width as u16),
                Ra::Constraint::Length(dialog_height as u16),
                f.area(),
            )
        };
        f.render_widget(Raw::Clear, area);
        let list = Raw::List::from_iter(self.items.iter().map(|i| {
            Raw::ListItem::new(i.chars().skip(self.offset).collect::<String>())
                .style(Ra::Style::default())
        }));
        f.render_stateful_widget(
            list.block(
                Raw::Block::default()
                    .borders(Raw::Borders::ALL)
                    .border_style(Ra::Style::default().fg(Theme::get().popup_block_fg))
                    .title(self.title.as_str()),
            )
            .highlight_style(
                Ra::Style::default()
                    .bg(Theme::get().list_hl_bg_fouced)
                    .add_modifier(Ra::Modifier::BOLD),
            ),
            area,
            &mut self.list_state,
        );

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
    /// close this by [EventState::Cancel]
    fn handle_key_event(
        &mut self,
        ev: &crossterm::event::KeyEvent,
    ) -> crate::tui::misc::EventState {
        if ev.kind != KeyEventKind::Press {
            return EventState::NotConsumed;
        }
        match ev.code {
            KeyCode::Down | KeyCode::Char('j') => self.next(),
            KeyCode::Up | KeyCode::Char('k') => self.previous(),
            KeyCode::Left | KeyCode::Char('h') => self.offset = self.offset.saturating_sub(1),
            KeyCode::Right | KeyCode::Char('l') => self.offset = self.offset.saturating_add(1),
            KeyCode::Esc | KeyCode::Char('n') => return EventState::Cancel,
            KeyCode::Enter | KeyCode::Char('y') => return EventState::Yes,
            KeyCode::Char('o') => return EventState::Choice2,
            KeyCode::Char('t') => return EventState::Choice3,
            _ => return EventState::NotConsumed,
        };
        EventState::WorkDone
    }
}

impl ListPopup {
    fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.scrollbar.next();
        self.list_state.select_next();
    }

    fn previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.scrollbar.prev();
        self.list_state.select_previous();
    }
}
