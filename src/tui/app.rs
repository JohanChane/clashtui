use super::*;
use tab::prelude::*;
use tokio::sync::Notify;
use widget::popmsg::PopUp;

// 50fps
const TICK_RATE: std::time::Duration = std::time::Duration::from_millis(20);
pub(super) static FULL_RENDER: Notify = Notify::const_new();

pub struct App {
    tabs: Vec<Tab>,
    // status_tab: StatusTab,
    // file_tab: FileTab,
    popup: PopUp,

    tab_index: u8,
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        Self {
            tabs: vec![StatusTab::default().into(), FileTab::default().into()],
            // status_tab: StatusTab::default(),
            // file_tab: FileTab::default(),
            popup: PopUp::default(),
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
    /// ``` md
    /// Keyevent
    ///     │  if popup
    ///     ├──────► PopUp
    ///     │  else
    ///     └─► App ─► Tab
    /// ```
    fn handle_key_event(&mut self, kv: &KeyEvent) {
        if self.popup.check() {
            self.popup.handle_key_event(kv);
        } else if !self.handle_global_kv(kv) {
            self.tabs[self.tab_index as usize].handle_key_event(kv);
        }
    }
    fn render(&mut self, f: &mut ratatui::Frame) {
        use ratatui::prelude::{Constraint, Layout};

        // split terminal into parts
        let chunks = Layout::default()
            .constraints([
                Constraint::Length(3),
                Constraint::Fill(1),
                // Constraint::Length(3),
            ])
            .split(f.area());

        render_tabbar(
            self.tabs.iter().map(|tab| tab.title()),
            self.tab_index,
            f,
            chunks[0],
        );

        self.tabs[self.tab_index as usize].render(f, chunks[1]);

        if self.popup.check() {
            self.popup.render(f, Default::default());
        }
    }
    /// This is the `App` layer, currently only handle Tab switch, Quitting
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
