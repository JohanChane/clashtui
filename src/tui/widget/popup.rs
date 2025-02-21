use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::{prelude as Ra, widgets as Raw};
use Ra::{Modifier, Style, Stylize};

use super::{tools, PopMsg, PopRes};
use crate::tui::misc::EventState;
use crate::tui::{Drawable, Theme};

const WRAP_TRUE: Raw::Wrap = Raw::Wrap { trim: true };

#[derive(Default)]
pub struct Popup {
    title: String,
    text: Option<Text>,
    input: Option<Input>,
    choices: Option<Choices>,
    focus: Focus,
}
impl Popup {
    pub fn reset(&mut self) {
        self.text = None;
        self.input = None;
        self.choices = None;
    }
    pub fn is_empty(&self) -> bool {
        self.text.is_none() && self.input.is_none() && self.choices.is_none()
    }
    pub fn show(&mut self, msg: PopMsg) {
        match msg {
            PopMsg::AskChoices(content, choices) => {
                self.title = "Msg".to_owned();
                self.focus = Focus::Text;
                self.text = Some(Text { content, offset: 0 });
                self.choices = Some(Choices { choices, index: 0 })
            }
            PopMsg::Prompt(content) => {
                self.title = "Msg".to_owned();
                self.focus = Focus::Text;
                self.text = Some(Text { content, offset: 0 })
            }
            PopMsg::SelectList(title, choices) => {
                self.title = title;
                self.focus = Focus::Extra;
                self.choices = Some(Choices { choices, index: 0 })
            }
            PopMsg::Input(title) => {
                self.title = title;
                self.focus = Focus::Extra;
                self.input = Some(Input::default())
            }
        }
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
        self.text.take();
        if let Some(chs) = self.choices.take() {
            Some(PopRes::Selected(chs.index))
        } else if let Some(ipt) = self.input.take() {
            Some(PopRes::Input(ipt.buffer))
        } else {
            None
        }
    }
    pub fn set_msg(&mut self, title: &str, items: Vec<String>) {
        self.title = title.to_string();
        self.focus = Focus::Text;
        let content = items.join("\n");
        self.text = Some(Text { content, offset: 0 });
    }
}
impl Drawable for Popup {
    fn render(&mut self, f: &mut ratatui::Frame, _: ratatui::layout::Rect, _: bool) {
        let area = |dialog_width: u16, dialog_height: u16| {
            // make up for block
            let dialog_width = (dialog_width + 2).min(f.area().width - 4);
            let dialog_height = (dialog_height + 2).min(f.area().height - 6);
            tools::centered_rect(
                Ra::Constraint::Length(dialog_width),
                Ra::Constraint::Length(dialog_height),
                f.area(),
            )
        };

        let first_block = Raw::Block::bordered()
            .border_style(Theme::get().popup.block)
            .title(self.title.as_str());
        let first_block_ext = {
            use Ra::symbols::{border, line::NORMAL};
            first_block.clone().border_set(border::Set {
                bottom_left: NORMAL.vertical_right,
                bottom_right: NORMAL.vertical_left,
                ..border::PLAIN
            })
        };
        let second_block = Raw::Block::bordered()
            .borders(Raw::Borders::ALL & !Raw::Borders::TOP)
            .border_style(Theme::get().popup.block);

        match (
            self.text.as_ref(),
            self.input.as_ref(),
            self.choices.as_ref(),
        ) {
            (None, None, None) | (None, Some(_), Some(_)) => unreachable!(),

            (None, Some(ipt), None) => {
                let area = area(ipt.width(), ipt.height());
                f.render_widget(Raw::Clear, area);

                let ipt = ipt
                    .widget(true)
                    .block(first_block)
                    // display the whole line while cursor moves
                    .scroll((0, (ipt.cursor as u16).saturating_sub(area.width - 8)));
                f.render_widget(ipt, area);
            }
            (None, None, Some(chs)) => {
                let area = area(chs.width(), chs.height());
                f.render_widget(Raw::Clear, area);

                let chs = chs.widget(true).block(first_block);
                f.render_widget(chs, area);
            }

            (Some(txt), None, None) => {
                let area = area(txt.width(), txt.height());
                f.render_widget(Raw::Clear, area);

                let para = txt.widget(true).block(first_block);
                f.render_widget(para, area);
            }

            (Some(txt), None, Some(chs)) => {
                let areas = {
                    let area = area(
                        txt.width().max(chs.width()),
                        txt.height() + chs.height() + 1,
                    );
                    f.render_widget(Raw::Clear, area);
                    Ra::Layout::vertical([
                        Ra::Constraint::Fill(1),
                        Ra::Constraint::Length(1 + chs.height()),
                    ])
                    .split(area)
                };

                let para = txt.widget(self.focus == Focus::Text).block(first_block_ext);
                f.render_widget(para, areas[0]);

                let chs = chs.widget(self.focus == Focus::Extra).block(second_block);
                f.render_widget(chs, areas[1]);
            }
            // ignore choices
            (Some(txt), Some(ipt), _) => {
                let areas = {
                    let area = area(
                        txt.width().max(ipt.width()),
                        txt.height() + ipt.height() + 1,
                    );
                    f.render_widget(Raw::Clear, area);

                    Ra::Layout::vertical([
                        Ra::Constraint::Fill(1),
                        Ra::Constraint::Length(1 + ipt.height()),
                    ])
                    .split(area)
                };

                let para = txt.widget(self.focus == Focus::Text).block(first_block_ext);
                f.render_widget(para, areas[0]);

                let ipt = ipt
                    .widget(self.focus == Focus::Extra)
                    .block(second_block)
                    .scroll((0, (ipt.cursor as u16).saturating_sub(areas[1].width - 8)));
                f.render_widget(ipt, areas[1]);
            }
        }
    }
    fn handle_key_event(&mut self, ev: &crossterm::event::KeyEvent) -> EventState {
        if ev.kind != KeyEventKind::Press {
            return EventState::NotConsumed;
        }
        let key = ev.code;
        match key {
            KeyCode::Tab => {
                if Focus::Text == self.focus {
                    self.focus = Focus::Extra
                } else {
                    self.focus = Focus::Text
                }
                EventState::WorkDone
            }
            KeyCode::Esc => EventState::Cancel,

            _ => match self.focus {
                Focus::Text => self.text.as_mut().unwrap().key(key),
                Focus::Extra => {
                    if let Some(ipt) = self.input.as_mut() {
                        ipt.key(key)
                    } else {
                        self.choices.as_mut().unwrap().key(key)
                    }
                }
            },
        }
    }
}

#[derive(Default, PartialEq)]
enum Focus {
    #[default]
    Text,
    Extra,
}

struct Text {
    pub content: String,
    pub offset: usize,
}
impl Text {
    #[inline]
    fn widget(&self, _is_focus: bool) -> Raw::Paragraph {
        Raw::Paragraph::new(self.content.as_str())
            .wrap(WRAP_TRUE)
            .scroll((self.offset as u16, 0))
    }
    #[inline]
    fn width(&self) -> u16 {
        Ra::Text::raw(&self.content).width() as u16
    }
    #[inline]
    fn height(&self) -> u16 {
        self.content.lines().count() as u16
    }
    #[inline]
    fn key(&mut self, key: KeyCode) -> EventState {
        match key {
            KeyCode::Down => self.offset += 1,
            KeyCode::Up => self.offset = self.offset.saturating_sub(1),
            _ => return EventState::NotConsumed,
        }
        EventState::WorkDone
    }
}

struct Choices {
    choices: Vec<String>,
    index: usize,
}
impl Choices {
    #[inline]
    fn widget(&self, is_focus: bool) -> Raw::Tabs {
        Raw::Tabs::new(self.choices.iter().map(|ch| ch.as_str()))
            .highlight_style(
                is_focus
                    .then_some(Style::default().add_modifier(Modifier::REVERSED))
                    .unwrap_or_default(),
            )
            .select(self.index)
    }
    #[inline]
    fn width(&self) -> u16 {
        self.choices
            .iter()
            .map(|s| s.len())
            .fold(0, |acc, len| acc + len + 3) as u16
            - 1
    }
    #[inline]
    fn height(&self) -> u16 {
        1
    }
    #[inline]
    fn key(&mut self, key: KeyCode) -> EventState {
        match key {
            KeyCode::Left => self.index = self.index.saturating_sub(1),
            KeyCode::Right => self.index = (self.index + 1).min(self.choices.len() - 1),

            KeyCode::Enter => {
                return EventState::Yes;
            }
            _ => return EventState::NotConsumed,
        }
        EventState::WorkDone
    }
}
#[derive(Default)]
struct Input {
    buffer: String,
    cursor: usize,
}
impl Input {
    #[inline]
    fn widget(&self, is_focus: bool) -> Raw::Paragraph {
        let mut before = format!("{} ", self.buffer);
        let mut after = before.split_off(self.cursor);
        let cursor = after.remove(0);
        Raw::Paragraph::new(Ra::Line::from_iter([
            Ra::Span::raw("> "),
            Ra::Span::raw(before),
            if is_focus {
                Ra::Span::raw(cursor.to_string()).add_modifier(Modifier::REVERSED)
            } else {
                Ra::Span::raw(cursor.to_string())
            },
            Ra::Span::raw(after),
        ]))
    }
    #[inline]
    fn width(&self) -> u16 {
        20
    }
    #[inline]
    fn height(&self) -> u16 {
        1
    }
    #[inline]
    fn key(&mut self, key: KeyCode) -> EventState {
        match key {
            KeyCode::Char(ch) => self.enter_char(ch),
            KeyCode::Backspace => self.delete_char(),
            KeyCode::Delete => self.delete_char_inplace(),
            KeyCode::Left => self.move_cursor_left(),
            KeyCode::Right => self.move_cursor_right(),

            KeyCode::Enter => {
                return EventState::Yes;
            }
            _ => return EventState::NotConsumed,
        }
        EventState::WorkDone
    }
}
impl Input {
    fn delete_char(&mut self) {
        if self.cursor != 0 {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.
            self.buffer = self
                .buffer
                .char_indices()
                .filter_map(|(idx, ch)| (idx != self.cursor - 1).then_some(ch))
                .collect();
            self.move_cursor_left();
        }
    }
    fn delete_char_inplace(&mut self) {
        self.buffer = self
            .buffer
            .char_indices()
            .filter_map(|(idx, ch)| (idx != self.cursor).then_some(ch))
            .collect();
        self.move_cursor_left();
    }
    fn enter_char(&mut self, ch: char) {
        self.buffer.insert(self.cursor, ch);
        self.move_cursor_right();
    }
    fn move_cursor_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    fn move_cursor_right(&mut self) {
        self.cursor = self.cursor.saturating_add(1).min(self.buffer.len());
    }
}
