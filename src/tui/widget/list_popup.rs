use crossterm::event::KeyCode;
use crossterm::event::KeyEventKind;
use ratatui::prelude as Ra;
use ratatui::widgets as Raw;

use super::input_popup::Item;
use super::tools;
use super::PopMsg;
use super::PopRes;
use crate::tui::misc::EventState;
use crate::tui::{Drawable, Theme};

enum Items {
    String(Vec<String>),
    NoFeedback(Vec<String>),
    Input(Vec<Item>),
}
impl Items {
    pub fn is_empty(&self) -> bool {
        match self {
            Items::NoFeedback(vec) | Items::String(vec) => vec.is_empty(),
            Items::Input(vec) => vec.is_empty(),
        }
    }
    pub fn clear(&mut self) {
        match self {
            Items::NoFeedback(vec) | Items::String(vec) => vec.clear(),
            Items::Input(vec) => vec.clear(),
        }
    }
    pub fn len(&self) -> usize {
        match self {
            Items::NoFeedback(vec) | Items::String(vec) => vec.len(),
            Items::Input(vec) => vec.len(),
        }
    }
}
impl Default for Items {
    fn default() -> Self {
        Self::NoFeedback(vec![])
    }
}

#[derive(Default)]
/// Pop a Message Window with line highlight
///
/// use arrow keys or `j\k\h\l`(vim-like) to navigate.
pub struct ListPopup {
    title: String,
    items: Items,
    __max_item_width: usize,
    state: Raw::ListState,
    scrollbar: Raw::ScrollbarState,
    offset: usize,
    selected: EventState,
}

impl ListPopup {
    /// ### Set Strings as message
    ///
    /// When call [collect](Self::collect), an enum will be produced
    /// to show which section are selected
    pub fn set_msg(&mut self, title: &str, items: Vec<String>, with_feedback: bool) {
        self.state = Default::default();
        self.offset = 0;
        self.title = title.to_owned();
        self.scrollbar = Raw::ScrollbarState::new(items.len());
        self.__max_item_width = items.iter().map(|i| i.len()).max().unwrap_or(0);
        self.items = if with_feedback {
            Items::String(items)
        } else {
            Items::NoFeedback(items)
        };
    }
    /// ### Set Strings as input request
    ///
    /// Every string will be regard as a question, and create a input instance.
    ///
    /// Multi-line text is not currently supported, as there is no need.
    ///
    /// When call [collect](Self::collect), an Vec will be produced
    /// to show user's answer.
    pub fn set_input(&mut self, items: Vec<String>) {
        self.state = Raw::ListState::default().with_selected(Some(0));
        self.offset = 0;
        self.title = "Input".to_owned();
        self.scrollbar = Raw::ScrollbarState::new(items.len());
        self.__max_item_width = 0;
        self.items = Items::Input(
            items
                .into_iter()
                .map(|title| Item::title(title.as_str()))
                .collect(),
        );
    }
    /// ### Return choice/input content
    /// in form of [Option]
    /// - [PopMsg::Prompt] provide no feedback
    /// - [PopMsg::Ask] will return an enum,
    /// - [PopMsg::Input] return [Vec]
    ///
    /// Should only be called when emiting [EventState::Yes],
    /// otherwise the return value is **UB**
    pub fn collect(&mut self) -> Option<PopRes> {
        match &mut self.items {
            Items::NoFeedback(..) => {
                self.clear();
                None
            }
            Items::String(..) => {
                self.clear();
                Some(PopRes::Selected(self.selected))
            }
            Items::Input(vec) => {
                let vec = std::mem::take(vec)
                    .into_iter()
                    .map(|itm| itm.content())
                    .collect();
                self.clear();
                Some(PopRes::Input(vec))
            }
        }
    }
    pub fn show_msg(&mut self, msg: PopMsg) {
        match msg {
            PopMsg::Ask(vec, chs) => {
                self.set_msg("Msg", vec, true);
                if let Items::String(vec) = &mut self.items {
                    vec.push(match chs.len() {
                        0 => "Press y for Yes, n for No".to_owned(),
                        1 => format!("Press y for Yes, n for No, o for {}", chs[0]),
                        2 => format!(
                            "Press y for Yes, n for No, o for {}, t for {}",
                            chs[0], chs[1]
                        ),
                        _ => unimplemented!("more than 2 extra choices!"),
                    });
                } else {
                    unreachable!()
                }
            }
            PopMsg::Prompt(vec) => {
                self.set_msg("Msg", vec, false);
                if let Items::NoFeedback(vec) = &mut self.items {
                    vec.push("Press Esc to close".to_owned());
                } else {
                    unreachable!()
                }
            }
            PopMsg::Input(vec) => self.set_input(vec),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    pub fn clear(&mut self) {
        self.items.clear();
    }
}

impl Drawable for ListPopup {
    /// No need to [Raw::clear], or plan aera
    fn render(&mut self, f: &mut ratatui::Frame, _: ratatui::layout::Rect, _: bool) {
        let area = {
            // let max_item_width = self.items.iter().map(|i| i.len()).max().unwrap_or(0);
            let dialog_width = (self.__max_item_width + 2)
                .min(f.area().width as usize - 4)
                .max(60); // min_width = 60
            let dialog_height = (self.items.len()
                * match &self.items {
                    Items::NoFeedback(..) | Items::String(..) => 1,
                    Items::Input(..) => 3,
                }
                + 2)
            .min(f.area().height as usize - 6);
            tools::centered_rect(
                Ra::Constraint::Length(dialog_width as u16),
                Ra::Constraint::Length(dialog_height as u16),
                f.area(),
            )
        };
        f.render_widget(Raw::Clear, area);

        let block = Raw::Block::default()
            .borders(Raw::Borders::ALL)
            .border_style(Ra::Style::default().fg(Theme::get().popup_block_fg))
            .title(self.title.as_str());
        match &mut self.items {
            Items::NoFeedback(vec) | Items::String(vec) => {
                let list = Raw::List::from_iter(vec.iter().map(|i| {
                    Raw::ListItem::new(i.chars().skip(self.offset).collect::<String>())
                        .style(Ra::Style::default())
                }));
                f.render_stateful_widget(
                    list.highlight_style(
                        Ra::Style::default()
                            .bg(Theme::get().list_hl_bg_fouced)
                            .add_modifier(Ra::Modifier::BOLD),
                    )
                    .block(block),
                    area,
                    &mut self.state,
                );
            }
            Items::Input(vec) => {
                let chunks = Ra::Layout::default()
                    .constraints([Ra::Constraint::Fill(1)].repeat(vec.len()))
                    .margin(1)
                    .split(area);

                // If the selected index is out of bounds, set it to the last item
                // this is done when rendering List, but there is no list in this branch
                if self.state.selected().is_some_and(|s| s >= vec.len()) {
                    self.state.select(Some(vec.len().saturating_sub(1)));
                }

                vec.iter_mut().enumerate().for_each(|(idx, itm)| {
                    itm.render(
                        f,
                        chunks[idx],
                        idx == self.state.selected().unwrap_or_default(),
                    )
                });
                f.render_widget(block, area);
            }
        }

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
        match &mut self.items {
            Items::NoFeedback(..) | Items::String(..) => match ev.code {
                KeyCode::Down | KeyCode::Char('j') => self.next(),
                KeyCode::Up | KeyCode::Char('k') => self.previous(),
                KeyCode::Left | KeyCode::Char('h') => self.offset = self.offset.saturating_sub(1),
                KeyCode::Right | KeyCode::Char('l') => self.offset = self.offset.saturating_add(1),
                KeyCode::Esc | KeyCode::Char('n') => {
                    self.selected = EventState::Cancel;
                    return EventState::Cancel;
                }
                KeyCode::Enter | KeyCode::Char('y') => {
                    self.selected = EventState::Yes;
                    return EventState::Yes;
                }
                KeyCode::Char('o') => {
                    self.selected = EventState::Choice2;
                    return EventState::Yes;
                }
                KeyCode::Char('t') => {
                    self.selected = EventState::Choice3;
                    return EventState::Yes;
                }
                _ => return EventState::NotConsumed,
            },
            Items::Input(vec) => match ev.code {
                KeyCode::Down => self.next(),
                KeyCode::Up => self.previous(),
                _ => {
                    return vec
                        .get_mut(self.state.selected().unwrap_or_default())
                        .unwrap()
                        .handle_key_event(ev)
                }
            },
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
        self.state.select_next();
    }

    fn previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.scrollbar.prev();
        self.state.select_previous();
    }
}
