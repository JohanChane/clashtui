use super::*;
use crate::tui::binding::DisplayBinding;
use crate::tui::global_keymap::GlobalAction;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use tab::prelude::*;
use tokio::sync::Notify;
use widget::chord::ChordHandler;
use widget::help::HelpPanel;
use widget::popmsg::PopUp;

use crossterm::event::KeyEventKind;

// 50fps
const TICK_RATE: std::time::Duration = std::time::Duration::from_millis(20);
pub(super) static FULL_RENDER: Notify = Notify::const_new();
pub(super) static SPINNER_FRAME: AtomicU8 = AtomicU8::new(0);
pub(crate) static QUIT: AtomicBool = AtomicBool::new(false);

/// Global display bindings -- initialized after global keymap is loaded.
pub(crate) static GLOBAL_DISPLAY_BINDINGS: LazyLock<Vec<DisplayBinding>> =
    LazyLock::new(|| crate::tui::global_keymap::get().to_display());

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
        if crate::config::CONFIG.cfg_file.mihomo.core_service.is_user {
            return;
        }
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
            terminal.draw(|f| app.render(f))?;
            app.sync();

            let ev = {
                use futures_lite::StreamExt as _;
                // this tick here ensures that fps is stable
                let mut tick = Box::pin(invt.tick());
                let ev = tokio::select! {
                    Some(ev) = events.next() => ev?,
                    _ = &mut tick => continue,
                    // if we switch between screens
                    _ = FULL_RENDER.notified() => {
                        // first we hold tui for output
                        FULL_RENDER.notified().await;
                        // then we tell ratatui to re-render everything
                        terminal.clear()?;
                        continue
                    },
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
                Event::Resize(..) => terminal.autoresize()?,
                _ => (),
            }
        }

        log::trace!("App Exit");
        Ok(())
    }

    /// KeyEvent Route:
    /// PopUp(0) -> Help(1) -> Tab scope(2) -> Global scope(3)
    fn handle_key_event(&mut self, kv: &Key) {
        log::debug!("K: {kv}");

        // Layer 0: PopUp -- dialogs hijack all input
        if self.popup.check() {
            self.popup.handle_key_event(kv);
            return;
        }

        // Layer 1: Help dismiss -- must be before Chord
        if self.help.is_active() {
            self.help.dismiss();
            return;
        }

        // Layer 2: Tab scope -- ChordHandler handles single keys + chords
        // SAFETY: shortcuts_ptr is a read-only slice reference derived from self.tabs.
        // The pointer is only used during the call to chord.handle(), and the dispatch
        // callback only accesses self.tabs by index (not by borrowing the same slice).
        let ti = self.tab_index as usize;
        let shortcuts_ptr: *const _ = self.tabs[ti].shortcuts() as *const _;
        if self
            .chord
            .handle(kv, unsafe { &*shortcuts_ptr }, &mut |seq| {
                log::debug!("chord dispatch: {seq:?}");
                self.tabs[self.tab_index as usize].dispatch_by_seq(seq);
            })
        {
            return;
        }

        // Layer 3: Global scope -- ChordHandler handles single keys + chords
        let mut global_action: Option<GlobalAction> = None;
        if self
            .global_chord
            .handle(kv, &GLOBAL_DISPLAY_BINDINGS, &mut |seq| {
                log::debug!("global chord dispatch: {seq:?}");
                if let Some(action) = crate::tui::global_keymap::get().find_by_seq(seq) {
                    global_action = Some(*action);
                }
            })
            && let Some(action) = global_action
        {
            self.dispatch_global_action(action);
        }
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
            self.render_which_for(f, &self.chord);
        }

        if self.global_chord.is_active() {
            self.render_which_for(f, &self.global_chord);
        }

        if self.help.is_active() {
            self.render_help(f, &self.tabs[self.tab_index as usize]);
        }

        if self.popup.check() {
            self.popup.render(f, Default::default());
        }
    }

    /// Unified Which? popup -- renders chord candidates from either tab or global chord handler.
    fn render_which_for(&self, f: &mut ratatui::Frame, chord: &ChordHandler) {
        use ratatui::layout::{Alignment, Constraint, Layout, Rect};
        use ratatui::style::Style;
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Clear, Paragraph};
        use widget::chord::key_event_to_str;

        let candidate_count = chord.candidates.len();
        let cols = if candidate_count > 4 { 2 } else { 1 };

        let total_height = candidate_count.div_ceil(cols) as u16 + 2;
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

        let items_per_col = candidate_count.div_ceil(cols);

        let accent = Theme::get().popup.text;

        for (col_idx, col_area) in col_areas.iter().enumerate().take(cols) {
            let lines: Vec<Line> = chord
                .candidates
                .iter()
                .skip(col_idx * items_per_col)
                .take(items_per_col)
                .map(|entry| {
                    let remaining = &entry.on[chord.pressed.len()..];
                    let key_str: String = remaining
                        .iter()
                        .map(key_event_to_str)
                        .collect::<Vec<_>>()
                        .join(" ");
                    Line::from(vec![
                        Span::raw(" "),
                        Span::styled(key_str, accent),
                        Span::raw("  "),
                        Span::styled(&entry.desc, Style::new().dim()),
                    ])
                })
                .collect();

            f.render_widget(Paragraph::new(lines), *col_area);
        }
    }

    fn render_help(&self, f: &mut ratatui::Frame, tab: &Tab) {
        widget::help::render_help(f, tab);
    }
    /// Dispatch a GlobalAction from the global keymap (Layer 3 routing).
    fn dispatch_global_action(&mut self, action: GlobalAction) {
        use GlobalAction::*;
        match action {
            GotoStatus => self.switch_tab(0),
            GotoFile => self.switch_tab(1),
            GotoProxies => self.switch_tab(2),
            GotoConnections => self.switch_tab(3),
            GotoLogs => self.switch_tab(4),
            GotoSettings => self.switch_tab(5),
            GotoService => self.switch_tab(6),
            CycleTab => {
                let n = self.tabs.len() as u8;
                let old = self.tab_index;
                self.tab_index = (old + 1) % n;
                if self.tab_index != old {
                    self.tabs[old as usize].on_leave();
                    self.tabs[self.tab_index as usize].on_enter();
                }
            }
            ToggleHelp => self.help.toggle(),
            Quit => {
                QUIT.store(true, Ordering::Relaxed);
            }
            OpenDataDir => {
                let dir = crate::config::core_data_dir(crate::config::CONFIG.core_type());
                let _ = crate::functions::command::open_dir(dir.to_str().unwrap());
            }
            OpenInstallDir => {
                let dir_str = match crate::config::CONFIG.core_type() {
                    crate::config::CoreType::Mihomo => {
                        &crate::config::CONFIG.cfg_file.mihomo.core.config_dir
                    }
                    crate::config::CoreType::Singbox => {
                        &crate::config::CONFIG.cfg_file.singbox.core.config_dir
                    }
                };
                let parent = std::path::Path::new(dir_str)
                    .parent()
                    .unwrap_or(std::path::Path::new(dir_str));
                let _ = crate::functions::command::open_dir(parent.to_str().unwrap());
            }
            RestartService => {
                let _ = crate::functions::command::restart_service();
            }
            TerminateAll => {
                let _ = crate::functions::restful::connection::terminate_all_connections();
            }
        }
    }

    fn switch_tab(&mut self, index: u8) {
        if index != self.tab_index {
            self.tabs[self.tab_index as usize].on_leave();
            self.tab_index = index;
            self.tabs[self.tab_index as usize].on_enter();
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
    use crate::tui::global_keymap::GlobalAction;

    fn mk_app() -> App {
        use std::sync::Once;
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            let tmp = std::env::temp_dir().join(format!("clashtui-test-{}", fastrand::u32(..)));
            std::fs::create_dir_all(&tmp).ok();
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                crate::config::init(Some(tmp)).unwrap()
            }));
        });

        App {
            tabs: vec![
                Tab::from(StatusTab::default()),
                Tab::from(FileTab::default()),
                Tab::from(ProxiesTab::default()),
                Tab::from(ConnectionsTab::default()),
                Tab::from(LogsTab::default()),
                Tab::from(SettingsTab::default()),
                Tab::from(CoreSrvCtlTab::default()),
            ],
            popup: PopUp::default(),
            chord: ChordHandler::default(),
            global_chord: ChordHandler::default(),
            help: HelpPanel::default(),
            tab_index: 0,
        }
    }

    #[test]
    fn switch_tab_1_to_2() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();
        let mut app = mk_app();
        QUIT.store(false, Ordering::Relaxed);
        app.dispatch_global_action(GlobalAction::GotoStatus);
        assert_eq!(app.tab_index, 0);
        app.dispatch_global_action(GlobalAction::GotoFile);
        assert_eq!(app.tab_index, 1);
    }

    #[test]
    fn switch_tab_to_service() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();
        let mut app = mk_app();
        QUIT.store(false, Ordering::Relaxed);
        app.dispatch_global_action(GlobalAction::GotoService);
        assert_eq!(app.tab_index, 6);
    }

    #[test]
    fn cycle_tab_forward() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();
        let mut app = mk_app();
        QUIT.store(false, Ordering::Relaxed);
        let tab_count = app.tabs.len() as u8;
        for i in 1..=tab_count {
            app.dispatch_global_action(GlobalAction::CycleTab);
            assert_eq!(app.tab_index, i % tab_count);
        }
    }

    #[test]
    fn quit_action() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();
        let mut app = mk_app();
        QUIT.store(false, Ordering::Relaxed);
        app.dispatch_global_action(GlobalAction::Quit);
        assert!(QUIT.load(Ordering::Relaxed));
        QUIT.store(false, Ordering::Relaxed);
    }

    #[test]
    fn help_toggle() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();
        let mut app = mk_app();
        QUIT.store(false, Ordering::Relaxed);
        assert!(!app.help.is_active());
        app.dispatch_global_action(GlobalAction::ToggleHelp);
        assert!(app.help.is_active());
        app.dispatch_global_action(GlobalAction::ToggleHelp);
        assert!(!app.help.is_active());
    }
}
