use super::*;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use tab::prelude::*;
use tokio::sync::Notify;
use widget::help::HelpPanel;
use widget::popmsg::PopUp;

// 50fps
const TICK_RATE: std::time::Duration = std::time::Duration::from_millis(20);
pub(super) static FULL_RENDER: Notify = Notify::const_new();
pub(super) static SPINNER_FRAME: AtomicU8 = AtomicU8::new(0);
pub(crate) static QUIT: AtomicBool = AtomicBool::new(false);

#[derive_aliases::derive(..Key)]
pub enum AppKey {
    Tab(u8),
    TabNext,
    Quit,
    Help,
}

pub(in crate::tui) mod km {
    //! We override the `set` and `get_docs` to keep
    //! the core key bindings under control
    use super::AppKey;
    use crate::tui::key::{Document, KeyDesc};
    use crossterm::event::KeyCode;

    // Core bindings are fixed and built directly in memory.
    key_map!(
        AppKey,
        DefaultMap::new().with_common(
            (2..=7u8)
                .map(|num| (KeyCode::Char((b'0' + num) as char), AppKey::Tab(num)))
                .chain(APP_KEYS)
        )
    );

    pub(super) use km::{cancel_pending, get_submap_name, pending_keys};

    pub fn set() -> anyhow::Result<()> {
        km::set(serde_yml::Value::Null)
    }

    pub fn get_docs() -> KeyDesc {
        km::get_docs()
            .into_iter()
            .filter(|(_, b)| *b != TAB_NUM)
            .chain(std::iter::once(("1~7".to_string(), TAB_NUM.to_string())))
            .collect()
    }

    const APP_KEYS: [(KeyCode, AppKey); 4] = [
        (KeyCode::Char('1'), AppKey::Tab(1)),
        (KeyCode::Tab, AppKey::TabNext),
        (KeyCode::Char('q'), AppKey::Quit),
        (KeyCode::Char('?'), AppKey::Help),
    ];
    const TAB_NUM: &str = "Switch tab 1-7";

    impl Document for AppKey {
        fn get_doc(&self) -> &'static str {
            match self {
                Self::Tab(..) => TAB_NUM,
                Self::TabNext => "Tab Next",
                Self::Quit => "Quit",
                Self::Help => "Help",
            }
        }
    }
}

pub struct App {
    tabs: Vec<Tab>,
    popup: PopUp,
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
            help: HelpPanel::default(),
            tab_index: 0,
        };
        app.tabs[0].on_enter();
        app
    }
    #[tokio::main]
    pub async fn serve() -> anyhow::Result<()> {
        signals::Signals::start()?;
        let mut app = Self::new();
        let mut events = crossterm::event::EventStream::new();
        let mut invt = tokio::time::interval(TICK_RATE);
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(std::io::stdout()))?;

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
                Event::Key(key_event) if key_event.kind.is_press() => {
                    #[cfg(debug_assertions)]
                    the_egg(key_event.code);
                    let key = key_event.into();
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
    /// PopUp → Help → Pending global chord → Active tab/chord → Global
    fn handle_key_event(&mut self, kv: &Key) {
        log::debug!("K: {kv:?}");

        if self.popup.check() {
            self.cancel_pending();
            self.popup.handle_key_event(kv);
            return;
        }

        if self.help.is_active() {
            self.cancel_pending();
            self.help.dismiss();
            return;
        }

        if km::pending_keys().is_some() {
            self.handle_global_kv(kv);
            return;
        }

        let ti = self.tab_index as usize;
        if !handle_tab_key(&mut self.tabs[ti], kv) {
            self.handle_global_kv(kv);
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

        if let Some(hints) =
            km::pending_keys().or_else(|| self.tabs[self.tab_index as usize].pending_keys())
        {
            widget::which::render(f, hints);
        }

        if self.help.is_active() {
            self.render_help(f, &self.tabs[self.tab_index as usize]);
        }

        if self.popup.check() {
            self.popup.render(f, Default::default());
        }
    }

    fn cancel_pending(&self) {
        km::cancel_pending();
        self.tabs[self.tab_index as usize].cancel_pending();
    }

    fn render_help(&self, f: &mut ratatui::Frame, tab: &Tab) {
        widget::help::render_help(f, tab);
    }
    /// Global layer (4) — last resort: Tab switch, Quit, Help
    fn handle_global_kv(&mut self, kv: &Key) -> bool {
        if km::pending_keys().is_none() && !km::km::has_binding(kv) {
            return false;
        }
        if let Ok(key) = AppKey::try_from(kv) {
            match key {
                AppKey::Tab(new_index) => {
                    let new_index = new_index - 1;
                    if new_index != self.tab_index {
                        self.cancel_pending();
                        self.tabs[self.tab_index as usize].on_leave();
                        self.tab_index = new_index;
                        self.tabs[self.tab_index as usize].on_enter();
                    }
                }
                AppKey::TabNext => {
                    self.cancel_pending();
                    const TAB_COUNT: u8 = 7;
                    let old_index = self.tab_index;
                    if self.tab_index == TAB_COUNT - 1 {
                        self.tab_index = 0;
                    } else {
                        self.tab_index += 1;
                    }
                    self.tabs[old_index as usize].on_leave();
                    self.tabs[self.tab_index as usize].on_enter();
                }
                AppKey::Quit => {
                    QUIT.store(true, Ordering::Relaxed);
                }
                AppKey::Help => self.help.toggle(),
            }
            true
        } else {
            false
        }
    }
    fn sync(&mut self) {
        SPINNER_FRAME.fetch_add(1, Ordering::Relaxed);
        self.popup.sync();
        if self.popup.check() {
            self.cancel_pending();
        }
        self.tabs.iter_mut().for_each(|tab| tab.sync());
    }
}

/// A prefix and its second key belong exclusively to the active scope,
/// including a mismatch or Esc. Never pass them to global shortcuts.
fn handle_tab_key(tab: &mut impl TuiTab, key: &Key) -> bool {
    let consumed = tab.pending_keys().is_some() || tab.has_binding(key);
    tab.handle_key_event(key);
    consumed
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
    let block = if let Some(submap_name) = km::get_submap_name() {
        block.title_bottom(submap_name)
    } else {
        block
    };
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
mod which_tests {
    use super::*;
    use crate::tui::key::{self, Document, KeyDesc, KeyMap, SubMap, WhichHints};
    use std::{cell::Cell, sync::OnceLock};

    #[derive(Clone, Copy, Eq, PartialEq, serde::Deserialize)]
    enum Action {
        Trigger,
    }
    impl Document for Action {
        fn get_doc(&self) -> &'static str {
            "Trigger"
        }
    }

    struct TestTab {
        pending: Cell<Option<&'static SubMap<Action>>>,
        fired: usize,
    }

    fn map() -> &'static KeyMap<Action> {
        static MAP: OnceLock<KeyMap<Action>> = OnceLock::new();
        MAP.get_or_init(|| key::config::merge(
            key::instancing::defaults::DefaultMap::new(),
            serde_yml::from_str("- {on: m, action: Trigger}\n- {on: x, action: Trigger}\n- {on: [g, q], action: Trigger}\n- {on: [g, '?'], action: Trigger}\n- {on: [g, '<Tab>'], action: Trigger}\n- {on: [q, x], action: Trigger}").unwrap()
        ).unwrap().0)
    }

    impl TuiWidget for TestTab {
        fn handle_key_event(&mut self, key: &Key) {
            if key::instancing::try_from(key, map(), self.pending.get_mut()).is_some() {
                self.fired += 1;
            }
        }
        fn render(&mut self, _: &mut ratatui::Frame, _: ratatui::layout::Rect) {}
        fn sync(&mut self) {}
    }
    impl TuiTab for TestTab {
        fn title(&self) -> &'static str {
            "Test"
        }
        fn key_description(&self) -> KeyDesc {
            vec![]
        }
        fn pending_keys(&self) -> Option<&'static WhichHints> {
            self.pending.get().map(|submap| &submap.hints)
        }
        fn cancel_pending(&self) {
            self.pending.set(None);
        }
        fn has_binding(&self, key: &Key) -> bool {
            map().contains_key(key)
        }
    }

    #[test]
    fn chord_prefixes_and_completions_do_not_fall_through_to_global_keys() {
        let mut tab = TestTab {
            pending: Cell::new(None),
            fired: 0,
        };
        for second in ["q", "?", "<Tab>"] {
            assert!(handle_tab_key(&mut tab, &"g".parse().unwrap()));
            assert!(tab.pending_keys().is_some());
            assert!(handle_tab_key(&mut tab, &second.parse().unwrap()));
            assert!(tab.pending_keys().is_none());
        }
        assert_eq!(tab.fired, 3);
        assert!(handle_tab_key(&mut tab, &"q".parse().unwrap()));
        assert!(tab.pending_keys().is_some());
        assert!(handle_tab_key(&mut tab, &"x".parse().unwrap()));
        assert_eq!(tab.fired, 4);
    }

    #[test]
    fn mismatch_and_escape_are_consumed_and_do_not_trigger_standalone_actions() {
        let mut tab = TestTab {
            pending: Cell::new(None),
            fired: 0,
        };
        for second in ["x", "<Esc>", "1"] {
            assert!(handle_tab_key(&mut tab, &"g".parse().unwrap()));
            assert!(handle_tab_key(&mut tab, &second.parse().unwrap()));
            assert!(tab.pending_keys().is_none());
            assert_eq!(tab.fired, 0);
        }
        assert!(handle_tab_key(&mut tab, &"m".parse().unwrap()));
        assert_eq!(tab.fired, 1);
        assert!(!handle_tab_key(&mut tab, &"?".parse().unwrap()));
        assert!(!handle_tab_key(&mut tab, &"<Tab>".parse().unwrap()));
    }

    #[test]
    fn cancelling_a_scope_discards_its_pending_prefix() {
        let mut tab = TestTab {
            pending: Cell::new(None),
            fired: 0,
        };
        handle_tab_key(&mut tab, &"g".parse().unwrap());
        tab.cancel_pending();
        assert!(tab.pending_keys().is_none());
        assert!(!handle_tab_key(&mut tab, &"?".parse().unwrap()));
        assert_eq!(tab.fired, 0);
    }
}
