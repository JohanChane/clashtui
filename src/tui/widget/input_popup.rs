use crossterm::event::KeyCode;
use ratatui::{
    prelude as Ra,
    widgets::{self as Raw, WidgetRef},
};

use crate::tui::{misc::EventState, Drawable, Theme};

/// A single input widget like
///```md
/// ┌title─────────────────────────────┐
/// │content                           │
/// └──────────────────────────────────┘
///```
pub struct Item {
    buffer: String,
    cursor: usize,
    title: String,
    is_highlight: bool,
}
impl Item {
    pub fn title(title: &str) -> Self {
        Self {
            title: title.to_string(),
            buffer: Default::default(),
            cursor: Default::default(),
            is_highlight: Default::default(),
        }
    }
    /// consume self and get content
    pub fn content(self) -> String {
        self.buffer
    }
}

impl WidgetRef for Item {
    fn render_ref(&self, area: Ra::Rect, buf: &mut Ra::Buffer) {
        Raw::Paragraph::new(self.buffer.as_str())
            .style(Ra::Style::default().fg(if self.is_highlight {
                Theme::get().input_text_selected_fg
            } else {
                Theme::get().input_text_unselected_fg
            }))
            .block(
                Raw::Block::default()
                    .borders(Raw::Borders::ALL)
                    .title(self.title.as_str()),
            )
            .render_ref(area, buf);
    }
}
impl Item {
    pub fn handle_key_code(&mut self, code: KeyCode) -> crate::tui::misc::EventState {
        match code {
            KeyCode::Char(ch) => self.enter_char(ch),
            KeyCode::Backspace => self.delete_char(),
            KeyCode::Left => self.move_cursor_left(),
            KeyCode::Right => self.move_cursor_right(),

            KeyCode::Enter => {
                self.reset_cursor();
                return EventState::Yes;
            }
            KeyCode::Esc => {
                self.reset_cursor();
                return EventState::Cancel;
            }
            _ => return EventState::NotConsumed,
        }
        EventState::WorkDone
    }
    fn reset_cursor(&mut self) {
        self.cursor = 0
    }
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
    fn enter_char(&mut self, ch: char) {
        self.buffer.insert(self.cursor, ch);
        self.move_cursor_right();
    }
    fn move_cursor_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1).clamp(0, self.buffer.len());
    }

    fn move_cursor_right(&mut self) {
        self.cursor = self.cursor.saturating_add(1).clamp(0, self.buffer.len());
    }
}

#[derive(Default)]
pub struct InputPopup {
    items: Vec<Item>,
    focus: usize,
}
impl InputPopup {
    pub fn with_msg(msg: Vec<String>) -> Self {
        Self {
            items: msg.into_iter().map(|s| Item::title(&s)).collect(),
            focus: 0,
        }
    }
    /// collect all contents, in the origin order
    pub fn collect(self) -> Vec<String> {
        self.items.into_iter().map(|s| s.content()).collect()
    }
}

impl Drawable for InputPopup {
    /// No need to [Raw::clear], or plan aera
    fn render(&mut self, f: &mut ratatui::Frame, _: ratatui::layout::Rect, _: bool) {
        use Ra::{Constraint, Layout};
        let input_area = Layout::default()
            .constraints([
                Constraint::Percentage(25),
                Constraint::Length(2 + self.items.len() as u16 * 3),
                Constraint::Min(0),
            ])
            .horizontal_margin(10)
            .vertical_margin(1)
            .split(f.area())[1];

        f.render_widget(Raw::Clear, input_area);

        let chunks = Ra::Layout::default()
            .constraints([Ra::Constraint::Fill(1)].repeat(self.items.len()))
            .margin(1)
            .split(input_area);

        self.items
            .iter_mut()
            .enumerate()
            .map(|(idx, itm)| {
                itm.is_highlight = idx == self.focus;
                (idx, itm)
            })
            .for_each(|(idx, itm)| itm.render_ref(chunks[idx], f.buffer_mut()));

        Raw::Block::new()
            .borders(Raw::Borders::ALL)
            .border_style(Ra::Style::default().fg(Ra::Color::Rgb(135, 206, 236)))
            .title("Input")
            .render_ref(input_area, f.buffer_mut());
    }
    /// this will not catch unrecognized key,
    /// which means key like `Tab` will still work.
    fn handle_key_event(
        &mut self,
        ev: &crossterm::event::KeyEvent,
    ) -> crate::tui::misc::EventState {
        if ev.kind != crossterm::event::KeyEventKind::Press {
            return EventState::NotConsumed;
        }
        match ev.code {
            KeyCode::Up => self.focus = self.focus.saturating_sub(1).clamp(0, self.items.len() - 1),
            KeyCode::Down => {
                self.focus = self.focus.saturating_add(1).clamp(0, self.items.len() - 1)
            }
            _ => {
                return self
                    .items
                    .get_mut(self.focus)
                    .unwrap()
                    .handle_key_code(ev.code)
            }
        }
        EventState::WorkDone
    }
}

#[test]
#[cfg(test)]
#[ignore = "used to preview widget"]
fn preview() {
    use crossterm::event::{KeyEvent, KeyModifiers};

    Theme::load(None).unwrap();
    fn df(f: &mut Ra::Frame) {
        let mut this = InputPopup::with_msg(vec![
            "test1".to_owned(),
            "test2".to_owned(),
            "test3".to_owned(),
        ]);
        this.handle_key_event(&KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        this.render(f, f.area(), true);
    }
    // let o = std::panic::take_hook();
    // std::panic::set_hook(Box::new(move |i| {
    //     let _ = setup::restore();
    //     o(i)
    // }));
    // use crate::tui::setup;
    // setup::setup().unwrap();
    let mut terminal = Ra::Terminal::new(Ra::CrosstermBackend::new(std::io::stdout())).unwrap();
    terminal.draw(|f| df(f)).unwrap();
    // std::thread::sleep(std::time::Duration::new(5, 0));
    // setup::restore().unwrap();
}
