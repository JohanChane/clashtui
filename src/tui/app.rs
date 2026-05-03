use super::*;
use tab::prelude::*;
use tokio::sync::Notify;
use widget::chord::ChordHandler;
use widget::popmsg::PopUp;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

// 50fps
const TICK_RATE: std::time::Duration = std::time::Duration::from_millis(20);
pub(super) static FULL_RENDER: Notify = Notify::const_new();

pub struct App {
    tabs: Vec<Tab>,
    popup: PopUp,
    chord: ChordHandler,

    tab_index: u8,
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        Self {
            tabs: vec![StatusTab::default().into(), FileTab::default().into()],
            popup: PopUp::default(),
            chord: ChordHandler::default(),
            tab_index: 0,
            should_quit: false,
        }
    }
    #[tokio::main]
    pub async fn serve() -> anyhow::Result<()> {
        let mut app = Self::new();
        let mut events = crossterm::event::EventStream::new();
        let mut invt = tokio::time::interval(TICK_RATE);
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(std::io::stdout()))?;

        while !app.should_quit {
            terminal.draw(|f| app.render(f))?;
            app.sync();

            let ev = {
                use futures_lite::StreamExt as _;
                // this tick here ensures that fps is stable
                let mut tick = Box::pin(invt.tick());
                let ev = tokio::select! {
                    Some(ev) = events.next() => ev?,
                    _ = &mut tick => continue,
                    // if we switch between screens, we have to tell ratatui to re-render everything
                    _ = FULL_RENDER.notified() => { terminal.clear()?; continue },
                };
                tick.await;
                ev
            };

            use crossterm::event::Event;
            match ev {
                Event::Key(key_event) => {
                    #[cfg(debug_assertions)]
                    the_egg(key_event.code);
                    app.handle_key_event(&key_event);
                }
                Event::Resize(..) => terminal.autoresize()?,
                _ => (),
            }
        }

        log::trace!("App Exit");
        Ok(())
    }

    /// KeyEvent Route:
    /// PopUp(0) → Which(1) → Tab(2) → Global(3)
    fn handle_key_event(&mut self, kv: &KeyEvent) {
        if self.popup.check() {
            self.popup.handle_key_event(kv);
            return;
        }

        let ti = self.tab_index as usize;
        let shortcuts_ptr: *const [(widget::tab::KeyCombo, &str)] = {
            self.tabs[ti].shortcuts() as *const _
        };

        if self.chord.handle(kv, unsafe { &*shortcuts_ptr }, &mut |seq| {
            self.tabs[ti].dispatch_shortcut(seq);
        }) {
            return;
        }

        self.tabs[ti].handle_key_event(kv);
        self.handle_global_kv(kv);
    }
    fn render(&mut self, f: &mut ratatui::Frame) {
        use ratatui::prelude::{Constraint, Layout};

        let chunks = Layout::default()
            .constraints([
                Constraint::Length(3),
                Constraint::Fill(1),
            ])
            .split(f.area());

        render_tabbar(
            self.tabs.iter().map(|tab| tab.title()),
            self.tab_index,
            f,
            chunks[0],
        );

        self.tabs[self.tab_index as usize].render(f, chunks[1]);

        if self.chord.is_active() {
            self.render_which(f);
        }

        if self.popup.check() {
            self.popup.render(f, Default::default());
        }
    }

    fn render_which(&self, f: &mut ratatui::Frame) {
        use ratatui::layout::{Alignment, Constraint, Layout, Rect};
        use ratatui::style::Stylize;
        use ratatui::text::Line;
        use ratatui::widgets::{Block, Clear, Paragraph};
        use widget::chord::key_event_to_str;

        let candidate_count = self.chord.candidates.len();
        let cols = if candidate_count > 4 { 2 } else { 1 };

        let total_height = ((candidate_count + cols - 1) / cols) as u16 + 2;
        let total_width = if cols == 1 { 40 } else { 70 };

        let area = f.area();
        let popup_area = Rect {
            x: area.x.saturating_add(area.width.saturating_sub(total_width) / 2),
            y: area.y.saturating_add(area.height.saturating_sub(total_height) / 2),
            width: total_width.min(area.width),
            height: total_height.min(area.height),
        };

        f.render_widget(Clear, popup_area);

        let block = Block::bordered()
            .title(" Which? ")
            .title_alignment(Alignment::Left);
        f.render_widget(block.clone(), popup_area);

        let inner = block.inner(popup_area);
        let col_widths: Vec<_> = (0..cols)
            .map(|_| Constraint::Ratio(1, cols as u32))
            .collect();
        let col_areas = Layout::horizontal(&col_widths).split(inner);

        let items_per_col = (candidate_count + cols - 1) / cols;

        for (col_idx, col_area) in col_areas.iter().enumerate().take(cols) {
            let lines: Vec<Line> = self
                .chord
                .candidates
                .iter()
                .skip(col_idx * items_per_col)
                .take(items_per_col)
                .map(|(seq, desc)| {
                    let remaining = &seq[self.chord.pressed.len()..];
                    let key_str: String = remaining
                        .iter()
                        .map(|k| key_event_to_str(k))
                        .collect::<Vec<_>>()
                        .join(" ");
                    Line::from(format!(" {}  {}", key_str, desc))
                })
                .collect();

            f.render_widget(Paragraph::new(lines), *col_area);
        }
    }
    /// Global layer (3) — last resort: Tab switch, Quit
    fn handle_global_kv(&mut self, kv: &KeyEvent) -> bool {
        if matches!(kv.kind, crossterm::event::KeyEventKind::Press) {
            use crossterm::event::KeyCode;
            const TAB_COUNT: u8 = 2;
            match kv.code {
                // Only match key we want
                KeyCode::Char(c @ '1'..='2') => self.tab_index = c as u8 - '1' as u8,
                KeyCode::Tab => {
                    if self.tab_index == TAB_COUNT - 1 {
                        self.tab_index = 0
                    } else {
                        self.tab_index += 1
                    }
                }
                KeyCode::Char('q') => self.should_quit = true,
                _ => return false,
            }
            return true;
        }
        false
    }
    fn sync(&mut self) {
        self.popup.sync();
        self.tabs.iter_mut().for_each(|tab| tab.sync());
    }
}

/// each item should represent for one tab
fn render_tabbar(
    titles: impl IntoIterator<Item = &'static str>,
    selected: u8,
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
) {
    use crate::tui::theme::Theme;
    use ratatui::style::{Styled, Stylize};
    use ratatui::text::Line;
    use ratatui::widgets::{Block, Tabs};

    let block = Block::bordered()
        .style(Theme::get().bars.block)
        .title(" Clashtui ")
        .title_bottom(Line::raw(" Tab or num ").right_aligned().reversed());
    let titles = titles
        .into_iter()
        .enumerate()
        .map(|(idx, s)| format!("{} {s}", idx + 1).set_style(Theme::get().bars.tabbar_text));
    let widget = Tabs::new(titles)
        .block(block)
        .highlight_style(Theme::get().bars.tabbar_highlight)
        .select(Some(selected as usize));
    f.render_widget(widget, area);
}

/// Ha! Magic Code!
#[cfg(debug_assertions)]
fn the_egg(key: crossterm::event::KeyCode) {
    use crossterm::event::KeyCode;
    static INSTANCE: std::sync::Mutex<u8> = std::sync::Mutex::new(0);
    let mut current = INSTANCE.lock().unwrap();
    match *current {
        0 | 1 if matches!(key, KeyCode::Up) => (),
        2 | 3 if matches!(key, KeyCode::Down) => (),
        4 | 6 if matches!(key, KeyCode::Left) => (),
        5 | 7 if matches!(key, KeyCode::Right) => (),
        8 | 10 if matches!(key, KeyCode::Char('b') | KeyCode::Char('B')) => (),
        9 | 11 if matches!(key, KeyCode::Char('a') | KeyCode::Char('A')) => (),
        _ => {
            *current = 0;
            return;
        }
    }
    *current += 1;
    if *current == 12 {
        log::debug!("You've found the egg!")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kev(code: KeyCode) -> KeyEvent {
        KeyEvent::new_with_kind_and_state(
            code,
            crossterm::event::KeyModifiers::empty(),
            KeyEventKind::Press,
            crossterm::event::KeyEventState::empty(),
        )
    }

    #[test]
    fn keyevent_vec_equals_slice() {
        let g = kev(KeyCode::Char('g'));
        let e = kev(KeyCode::Char('e'));

        let vec: Vec<KeyEvent> = vec![g, e];
        let slice: &[KeyEvent] = &[g, e];

        assert_eq!(vec, slice);
        assert_eq!(&vec, slice);
    }

    #[test]
    fn keyevents_compare_equal() {
        let a = kev(KeyCode::Char('e'));
        let b = kev(KeyCode::Char('e'));
        assert_eq!(a, b);
    }
}
