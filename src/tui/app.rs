use super::*;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use tab::prelude::*;
use tokio::sync::Notify;
use widget::chord::ChordHandler;
use widget::help::HelpPanel;
use widget::popmsg::PopUp;

use Key;
use crossterm::event::{KeyCode, KeyEventKind};
use widget::tab::KeyCombo;

// 50fps
const TICK_RATE: std::time::Duration = std::time::Duration::from_millis(20);
pub(super) static FULL_RENDER: Notify = Notify::const_new();
pub(super) static SPINNER_FRAME: AtomicU8 = AtomicU8::new(0);
pub(crate) static QUIT: AtomicBool = AtomicBool::new(false);
pub(crate) static RESIZE: AtomicBool = AtomicBool::new(false);

static GLOBAL_CHORD_SHORTCUTS: LazyLock<Vec<(KeyCombo, &str)>> = LazyLock::new(|| {
    fn ctrl(c: char) -> Key {
        Key {
            code: KeyCode::Char(c),
            shift: false,
            ctrl: true,
            alt: false,
            super_: false,
        }
    }
    fn plain(c: char) -> Key {
        Key {
            code: KeyCode::Char(c),
            shift: false,
            ctrl: false,
            alt: false,
            super_: false,
        }
    }
    vec![
        (KeyCombo(vec![ctrl('g'), plain('c')]), "Open app config dir"),
        (
            KeyCombo(vec![ctrl('g'), plain('m')]),
            "Open clash config dir",
        ),
        (KeyCombo(vec![ctrl('g'), plain('f')]), "Start core service"),
        (
            KeyCombo(vec![ctrl('g'), plain('t')]),
            "Close all connections",
        ),
    ]
});

pub struct App {
    tabs: Vec<Tab>,
    popup: PopUp,
    chord: ChordHandler,
    global_chord: ChordHandler,
    help: HelpPanel,

    tab_index: u8,
}

impl App {
    fn new() -> Self {
        let mut app = Self {
            tabs: vec![
                StatusTab::default().into(),
                FileTab::default().into(),
                ProxiesTab::default().into(),
                ConnectionsTab::default().into(),
                LogsTab::default().into(),
                SettingsTab::default().into(),
                CoreSrvCtlTab::default().into(),
            ],
            popup: PopUp::default(),
            chord: ChordHandler::default(),
            global_chord: ChordHandler::default(),
            help: HelpPanel::default(),
            tab_index: 0,
        };
        app.tabs[0].on_enter();
        app
    }
    #[cfg(target_family = "unix")]
    fn check_startup_perms(&self) {
        use std::io::Write;

        let dirs_to_check = [
            &crate::config::CONFIG.cfg_file.mihomo.core.config_dir,
            &crate::config::CONFIG.cfg_file.singbox.core.config_dir,
        ];

        for dir_str in &dirs_to_check {
            if dir_str.is_empty() {
                continue;
            }
            let dir = std::path::Path::new(dir_str);
            if !dir.exists() {
                continue;
            }
            if crate::functions::command::check_file_permissions(dir) {
                continue;
            }

            let _ = crate::tui::hold(true);
            print!(
                "File permissions in '{}' need repair. Fix now? [Y/n] ",
                dir.display()
            );
            let _ = std::io::stdout().flush();
            let mut input = String::new();
            let _ = std::io::stdin().read_line(&mut input);
            let _ = crate::tui::hold(false);

            if input.trim().to_lowercase().as_str() != "y" {
                continue;
            }

            let Some(group) = crate::functions::command::get_dir_group_name(dir) else {
                continue;
            };

            if let Err(e) = crate::functions::command::repair_file_permissions(dir, &group) {
                let _ = crate::tui::hold(true);
                eprintln!("Error: {}", e);
                use std::io::Read;
                print!("Press Enter to continue...");
                let _ = std::io::stdout().flush();
                let _ = std::io::stdin().read(&mut [0u8]);
                let _ = crate::tui::hold(false);
            }
        }
    }
    #[cfg(not(target_family = "unix"))]
    fn check_startup_perms(&self) {}
    #[tokio::main]
    pub async fn serve() -> anyhow::Result<()> {
        signals::Signals::start()?;
        let mut app = Self::new();
        let mut events = crossterm::event::EventStream::new();
        let mut invt = tokio::time::interval(TICK_RATE);
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(std::io::stdout()))?;

        app.check_startup_perms();
        while !QUIT.load(Ordering::Relaxed) {
            if crate::tui::EXT_PROC.load(Ordering::SeqCst) {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                continue;
            }

            if RESIZE.swap(false, Ordering::Relaxed) {
                terminal.autoresize()?;
            }

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
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    #[cfg(debug_assertions)]
                    the_egg(key_event.code);
                    let key: Key = key_event.into();
                    app.handle_key_event(&key);
                }
                Event::Resize(..) => {
                    RESIZE.store(true, Ordering::Relaxed);
                }
                _ => (),
            }
        }

        log::trace!("App Exit");
        Ok(())
    }

    /// KeyEvent Route:
    /// PopUp(0) → GlobalChord(0.5) → Help(1) → Which(2) → Tab(3) → Global(4)
    fn handle_key_event(&mut self, kv: &Key) {
        log::debug!("K: {kv}");

        if self.popup.check() {
            self.popup.handle_key_event(kv);
            return;
        }

        {
            let shortcuts_ptr: *const [(KeyCombo, &str)] =
                GLOBAL_CHORD_SHORTCUTS.as_slice() as *const _;
            if self
                .global_chord
                .handle(kv, unsafe { &*shortcuts_ptr }, &mut |seq| {
                    log::debug!("global_chord dispatch: {seq:?}");
                    match seq.last().and_then(|k| k.plain()) {
                        Some('c') => {
                            log::debug!("open_dir: config dir");
                            let _ = crate::functions::command::open_dir(
                                crate::config::config_root_path().to_str().unwrap(),
                            );
                        }
                        Some('m') => {
                            log::debug!("open_dir: clash config dir");
                            let dir_str = match crate::config::CONFIG.core_type() {
                                crate::config::CoreType::Mihomo => {
                                    &crate::config::CONFIG.cfg_file.mihomo.core.config_dir
                                }
                                crate::config::CoreType::Singbox => {
                                    &crate::config::CONFIG.cfg_file.singbox.core.config_dir
                                }
                            };
                            let _ = crate::functions::command::open_dir(dir_str);
                        }
                        Some('f') => {
                            log::debug!("restart core service");
                            let _ = crate::functions::command::restart_service(None);
                        }
                        Some('t') => {
                            log::debug!("close all connections");
                            let _ =
                                crate::functions::restful::connection::terminate_all_connections();
                        }
                        _ => {}
                    }
                })
            {
                return;
            }
        }

        if self.help.is_active() {
            self.help.dismiss();
            return;
        }

        let ti = self.tab_index as usize;
        let shortcuts_ptr: *const [(widget::tab::KeyCombo, &str)] =
            { self.tabs[ti].shortcuts() as *const _ };

        if self
            .chord
            .handle(kv, unsafe { &*shortcuts_ptr }, &mut |seq| {
                log::debug!("chord dispatch: {seq:?}");
                self.tabs[ti].dispatch_shortcut(seq);
            })
        {
            return;
        }

        self.tabs[ti].handle_key_event(kv);
        self.handle_global_kv(kv);
    }
    fn render(&mut self, f: &mut ratatui::Frame) {
        use ratatui::prelude::{Constraint, Layout};

        let chunks = Layout::default()
            .constraints([Constraint::Length(3), Constraint::Fill(1)])
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

        if self.global_chord.is_active() {
            self.render_global_which(f);
        }

        if self.help.is_active() {
            self.render_help(f, &self.tabs[self.tab_index as usize]);
        }

        if self.popup.check() {
            self.popup.render(f, Default::default());
        }
    }

    fn render_which(&self, f: &mut ratatui::Frame) {
        use ratatui::layout::{Alignment, Constraint, Layout, Rect};
        use ratatui::style::{Style, Stylize};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Clear, Paragraph};
        use widget::chord::key_event_to_str;

        let candidate_count = self.chord.candidates.len();
        let cols = if candidate_count > 4 { 2 } else { 1 };

        let total_height = ((candidate_count + cols - 1) / cols) as u16 + 2;
        let total_width = if cols == 1 { 40 } else { 70 };

        let area = f.area();
        let popup_area = Rect {
            x: area
                .x
                .saturating_add(area.width.saturating_sub(total_width) / 2),
            y: area.height.saturating_sub(total_height + 2),
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

        let accent = Theme::get().popup.text;

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
                    Line::from(vec![
                        Span::raw(" "),
                        Span::styled(key_str, accent),
                        Span::raw("  "),
                        Span::styled(*desc, Style::new().dim()),
                    ])
                })
                .collect();

            f.render_widget(Paragraph::new(lines), *col_area);
        }
    }

    fn render_global_which(&self, f: &mut ratatui::Frame) {
        use ratatui::layout::{Alignment, Constraint, Layout, Rect};
        use ratatui::style::{Style, Stylize};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Clear, Paragraph};
        use widget::chord::key_event_to_str;

        let candidate_count = self.global_chord.candidates.len();
        let cols = if candidate_count > 4 { 2 } else { 1 };

        let total_height = ((candidate_count + cols - 1) / cols) as u16 + 2;
        let total_width = if cols == 1 { 40 } else { 70 };

        let area = f.area();
        let popup_area = Rect {
            x: area
                .x
                .saturating_add(area.width.saturating_sub(total_width) / 2),
            y: area.height.saturating_sub(total_height + 2),
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

        let accent = Theme::get().popup.text;

        for (col_idx, col_area) in col_areas.iter().enumerate().take(cols) {
            let lines: Vec<Line> = self
                .global_chord
                .candidates
                .iter()
                .skip(col_idx * items_per_col)
                .take(items_per_col)
                .map(|(seq, desc)| {
                    let remaining = &seq[self.global_chord.pressed.len()..];
                    let key_str: String = remaining
                        .iter()
                        .map(|k| key_event_to_str(k))
                        .collect::<Vec<_>>()
                        .join(" ");
                    Line::from(vec![
                        Span::raw(" "),
                        Span::styled(key_str, accent),
                        Span::raw("  "),
                        Span::styled(*desc, Style::new().dim()),
                    ])
                })
                .collect();

            f.render_widget(Paragraph::new(lines), *col_area);
        }
    }

    fn render_help(&self, f: &mut ratatui::Frame, tab: &Tab) {
        widget::help::render_help(f, tab);
    }
    /// Global layer (4) — last resort: Tab switch, Quit, Help
    fn handle_global_kv(&mut self, kv: &Key) -> bool {
        const TAB_COUNT: u8 = 7;
        match kv.code {
            KeyCode::Char(c @ '1'..='7') if !kv.ctrl && !kv.alt && !kv.super_ => {
                let new_index = c as u8 - '1' as u8;
                if new_index != self.tab_index {
                    self.tabs[self.tab_index as usize].on_leave();
                    self.tab_index = new_index;
                    self.tabs[self.tab_index as usize].on_enter();
                }
                return true;
            }
            KeyCode::Tab if !kv.ctrl && !kv.alt && !kv.super_ => {
                let old_index = self.tab_index;
                if self.tab_index == TAB_COUNT - 1 {
                    self.tab_index = 0;
                } else {
                    self.tab_index += 1;
                }
                if self.tab_index != old_index {
                    self.tabs[old_index as usize].on_leave();
                    self.tabs[self.tab_index as usize].on_enter();
                }
                return true;
            }
            KeyCode::Char('q') if !kv.ctrl && !kv.alt && !kv.super_ => {
                QUIT.store(true, Ordering::Relaxed);
                return true;
            }
            KeyCode::Char('c') if kv.ctrl && !kv.alt && !kv.super_ => {
                QUIT.store(true, Ordering::Relaxed);
                return true;
            }
            KeyCode::Char('?') if !kv.ctrl && !kv.alt && !kv.super_ => {
                self.help.toggle();
                return true;
            }
            _ => false,
        }
    }
    fn sync(&mut self) {
        SPINNER_FRAME.fetch_add(1, Ordering::Relaxed);
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

    let theme = Theme::get();
    let block = Block::bordered()
        .title(" Clashtui ")
        .title_bottom(Line::raw(" Tab or num ").right_aligned().reversed());
    let titles = titles
        .into_iter()
        .enumerate()
        .map(|(idx, s)| format!("{} {s}", idx + 1).set_style(theme.tabbar.text));
    let widget = Tabs::new(titles)
        .block(block)
        .highlight_style(theme.tabbar.highlight)
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

    fn kev(code: KeyCode) -> Key {
        Key {
            code,
            shift: false,
            ctrl: false,
            alt: false,
            super_: false,
        }
    }

    #[test]
    fn keyevent_vec_equals_slice() {
        let g = kev(KeyCode::Char('g'));
        let e = kev(KeyCode::Char('e'));

        let vec: Vec<Key> = vec![g, e];
        let slice: &[Key] = &[g, e];

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
