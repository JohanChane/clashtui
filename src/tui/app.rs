mod actions;

use super::business::{Business, Completion};
use super::core::{ApiResource, ServiceOperation};
use super::*;
pub(in crate::tui) use actions::{AppAction, TabTarget, km};
use std::{
    future::poll_fn,
    task::Poll,
    time::{Duration, Instant},
};
use tab::prelude::*;
use tokio::sync::Notify;
use widget::help::HelpPanel;
use widget::popmsg::PopUp;

static QUIT_REQUESTED: Notify = Notify::const_new();
const LOG_REFRESH_INTERVAL: Duration = Duration::from_millis(34);

fn schedule_log_refresh(visible: bool, refresh_at: &mut Option<Instant>, now: Instant) {
    if visible {
        refresh_at.get_or_insert(now + LOG_REFRESH_INTERVAL);
    }
}

fn take_log_refresh(visible: bool, refresh_at: &mut Option<Instant>, now: Instant) -> bool {
    if !visible {
        *refresh_at = None;
        return false;
    }
    if refresh_at.is_some_and(|at| at <= now) {
        *refresh_at = None;
        true
    } else {
        false
    }
}

pub(crate) fn request_quit() {
    QUIT_REQUESTED.notify_one();
}

pub struct App {
    tabs: Vec<Tab>,
    popup: PopUp,
    help: HelpPanel,
    input: key::Matcher<'static>,

    tab_index: u8,
    business: Business,
    log_refresh_at: Option<Instant>,
    globals: tokio::task::JoinSet<anyhow::Result<()>>,
    quitting: bool,
    dirty: bool,
}

impl App {
    fn new() -> anyhow::Result<Self> {
        let tabs: Vec<Tab> = vec![
            StatusTab::default().into(),
            FileTab::default().into(),
            ProxiesTab::default().into(),
            ConnectionsTab::default().into(),
            LogsTab::default().into(),
            SettingsTab::default().into(),
            CoreSrvCtlTab::default().into(),
        ];
        let mut app = Self {
            tabs,
            popup: PopUp::default(),
            help: HelpPanel::default(),
            input: key::Matcher::new(key::keymaps()),
            tab_index: 0,
            business: Business::default(),
            log_refresh_at: None,
            globals: tokio::task::JoinSet::new(),
            quitting: false,
            dirty: true,
        };
        app.with_tab(0, |tab, cx| tab.on_enter(cx));
        app.sync_visible();
        Ok(app)
    }

    fn with_tab<R>(&mut self, index: usize, f: impl FnOnce(&mut Tab, &mut Business) -> R) -> R {
        let (result, messages) =
            widget::popmsg::with_local_messages(|| f(&mut self.tabs[index], &mut self.business));
        for message in messages {
            self.popup.accept(message);
            self.dirty = true;
        }
        result
    }

    fn logs_visible(&self) -> bool {
        !self.quitting && matches!(self.tabs[self.tab_index as usize], Tab::LogsTab(_))
    }

    /// Only the visible ordinary view is synchronized. Files advances both pane loads.
    fn sync_visible(&mut self) {
        if self.quitting || self.logs_visible() {
            return;
        }
        let tab = &mut self.tabs[self.tab_index as usize];
        tab.advance();
        tab.sync_from(self.business.view());
        self.dirty = true;
    }

    fn refresh_logs_if_due(&mut self, now: Instant) {
        if take_log_refresh(self.logs_visible(), &mut self.log_refresh_at, now) {
            self.tabs[self.tab_index as usize].sync_from(self.business.view());
            self.dirty = true;
        }
    }

    fn show_feedback(&mut self, messages: Vec<String>) {
        let (_, popups) = widget::popmsg::with_local_messages(|| {
            for message in messages {
                if self.quitting {
                    log::warn!("Shutdown result: {message}");
                } else {
                    log::info!("{message}");
                }
                if !self.quitting {
                    popmsg::prelude::Confirm::notification("Operation result".into())
                        .with_prompt(message)
                        .build_and_send();
                }
            }
        });
        for popup in popups {
            self.popup.accept(popup);
            self.dirty = true;
        }
    }

    fn begin_shutdown(&mut self, terminal: &mut term::TerminalCoordinator) {
        if self.quitting {
            return;
        }
        self.quitting = true;
        self.business.begin_shutdown();
        self.with_tab(self.tab_index as usize, |tab, cx| tab.on_leave(cx));
        self.log_refresh_at = None;
        self.popup.begin_shutdown();
        terminal.begin_shutdown();
    }

    fn shutdown_finished(&self, terminal: &term::TerminalCoordinator) -> bool {
        self.quitting
            && !self.business.has_tasks()
            && !self.business.has_log_worker()
            && self.globals.is_empty()
            && self.tabs.iter().all(|tab| !tab.has_tasks())
            && terminal.is_idle()
    }

    fn handle_terminal_control(
        &mut self,
        coordinator: &mut term::TerminalCoordinator,
        control: term::TerminalControl,
        input: &mut term::InputReader,
        terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    ) -> anyhow::Result<()> {
        let clear = coordinator.handle(control)?;
        if coordinator.owns_terminal() && !self.quitting {
            input.resume();
            if clear {
                terminal.clear()?;
            }
            self.dirty = true;
        }
        Ok(())
    }

    #[tokio::main]
    pub async fn serve() -> anyhow::Result<()> {
        let mut coordinator = term::TerminalCoordinator::install();
        let signals = signals::Signals::start()?;
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(std::io::stdout()))?;
        let mut app = Self::new()?;
        let mut input = term::InputReader::start();
        let mut pause_ack = None;
        let mut pending_control = None;
        let mut input_stopping = false;
        let log_notification = app.business.log_notification();
        let mut animation_at: Option<Instant> = None;
        let mut input_open = true;
        let mut popup_open = true;
        let mut control_open = true;
        let mut failure = None;

        enum Wake {
            Input(Option<std::io::Result<crossterm::event::Event>>),
            Popup(Option<widget::popmsg::Wrapped>),
            TabTask(usize),
            Business(Completion),
            LogChanged,
            Global(Option<Result<anyhow::Result<()>, tokio::task::JoinError>>),
            Due,
            Terminal(Option<term::TerminalControl>),
            InputPaused(Result<(), tokio::sync::oneshot::error::RecvError>),
            Quit,
        }
        loop {
            let feedback = app.business.take_feedback();
            app.show_feedback(feedback);
            app.refresh_logs_if_due(Instant::now());
            if !app.quitting {
                if app.dirty && coordinator.owns_terminal() {
                    match terminal.draw(|frame| app.render(frame)) {
                        Ok(_) => app.dirty = false,
                        Err(error) => {
                            failure = Some(error.into());
                            app.begin_shutdown(&mut coordinator);
                        }
                    }
                }
                if app.tabs[app.tab_index as usize].needs_animation() {
                    animation_at.get_or_insert_with(|| Instant::now() + Duration::from_millis(160));
                } else {
                    animation_at = None;
                }
            }
            if app.quitting && !input_stopping {
                input.begin_shutdown();
                input_stopping = true;
            }
            if app.shutdown_finished(&coordinator)
                && !input.has_worker()
                && pending_control.is_none()
            {
                break;
            }
            let deadline = if app.quitting {
                None
            } else {
                app.business
                    .next_deadline()
                    .into_iter()
                    .chain(animation_at)
                    .chain(app.log_refresh_at)
                    .min()
            };
            let wake = tokio::select! {
                event = input.next_event(), if input_open => Wake::Input(event),
                paused = async { pause_ack.as_mut().unwrap().await }, if pause_ack.is_some() => Wake::InputPaused(paused),
                popup = app.popup.next_message(), if popup_open => Wake::Popup(popup),
                index = poll_fn(|cx| {
                    for (index, tab) in app.tabs.iter_mut().enumerate() {
                        if tab.poll_tasks(cx).is_ready() { return Poll::Ready(index); }
                    }
                    Poll::Pending
                }), if app.tabs.iter().any(|tab| tab.has_tasks()) => Wake::TabTask(index),
                done = poll_fn(|cx| app.business.poll_completion(cx)), if app.business.has_work() => Wake::Business(done),
                _ = log_notification.notified() => Wake::LogChanged,
                done = app.globals.join_next(), if !app.globals.is_empty() => Wake::Global(done),
                _ = async { if let Some(deadline) = deadline { tokio::time::sleep_until(deadline.into()).await; } else { std::future::pending::<()>().await; } } => Wake::Due,
                control = coordinator.recv(), if control_open => Wake::Terminal(control),
                _ = QUIT_REQUESTED.notified(), if !app.quitting => Wake::Quit,
            };
            match wake {
                Wake::Input(Some(Ok(event)))
                    if !app.quitting
                        && coordinator.owns_terminal()
                        && pending_control.is_none() =>
                {
                    match event {
                        crossterm::event::Event::Key(key_event) if key_event.kind.is_press() => {
                            #[cfg(debug_assertions)]
                            the_egg(key_event.code);
                            app.handle_key_event(&key_event.into());
                        }
                        crossterm::event::Event::Resize(..) => {
                            if let Err(error) = terminal.autoresize() {
                                failure = Some(error.into());
                                app.begin_shutdown(&mut coordinator);
                            }
                        }
                        _ => {}
                    }
                    app.dirty = true;
                }
                Wake::Input(Some(Ok(_))) => {}
                Wake::Input(Some(Err(error))) => {
                    failure = Some(error.into());
                    app.begin_shutdown(&mut coordinator);
                }
                Wake::Input(None) => {
                    input_open = false;
                    app.begin_shutdown(&mut coordinator);
                }
                Wake::Popup(Some(message)) => {
                    app.popup.accept(message);
                    app.input.cancel();
                    app.sync_visible();
                    app.dirty = true;
                }
                Wake::Popup(None) => popup_open = false,
                Wake::TabTask(index) => {
                    let visible = !app.quitting && index == app.tab_index as usize;
                    let (changed, messages) = widget::popmsg::with_local_messages(|| {
                        app.tabs[index].finish_task(visible)
                    });
                    app.dirty |= changed && visible;
                    app.sync_visible();
                    for message in messages {
                        app.popup.accept(message);
                        app.dirty = true;
                    }
                }
                Wake::Global(Some(result)) => {
                    let result = result.unwrap_or_else(|error| Err(error.into()));
                    if let Err(error) = result {
                        app.show_feedback(vec![format!("{error:#}")]);
                    }
                    app.sync_visible();
                }
                Wake::Business(Completion::Core) => app.sync_visible(),
                Wake::Global(None) | Wake::Business(Completion::Logs) => {}
                Wake::LogChanged => {
                    schedule_log_refresh(
                        app.logs_visible(),
                        &mut app.log_refresh_at,
                        Instant::now(),
                    );
                }
                Wake::Due => {
                    app.business.run_due();
                    app.sync_visible();
                    if animation_at.is_some_and(|at| at <= Instant::now()) {
                        app.dirty = true;
                        animation_at = None;
                    }
                }
                Wake::Terminal(Some(control)) => {
                    if control.is_suspend() {
                        pause_ack = Some(input.request_pause());
                        pending_control = Some(control);
                    } else if let Err(error) = app.handle_terminal_control(
                        &mut coordinator,
                        control,
                        &mut input,
                        &mut terminal,
                    ) {
                        failure = Some(error);
                        app.begin_shutdown(&mut coordinator);
                    }
                }
                Wake::InputPaused(result) => {
                    pause_ack = None;
                    if result.is_err() && !app.quitting {
                        failure = Some(anyhow::anyhow!(
                            "Terminal input stopped before confirming suspension"
                        ));
                        app.begin_shutdown(&mut coordinator);
                    }
                    if let Some(control) = pending_control.take()
                        && let Err(error) = app.handle_terminal_control(
                            &mut coordinator,
                            control,
                            &mut input,
                            &mut terminal,
                        )
                    {
                        failure = Some(error);
                        app.begin_shutdown(&mut coordinator);
                    }
                }
                Wake::Terminal(None) => {
                    control_open = false;
                    app.begin_shutdown(&mut coordinator);
                }
                Wake::Quit => app.begin_shutdown(&mut coordinator),
            }
        }
        drop(input);
        signals.shutdown().await;
        log::trace!("App Exit");
        failure.map_or(Ok(()), Err)
    }

    /// Popups capture text input; all page/global shortcuts share one compiled map.
    /// Help can start a global-only chord, which retains its candidates until completion.
    fn handle_key_event(&mut self, kv: &Key) {
        log::debug!("K: {kv:?}");
        if self.popup.check() {
            self.input.cancel();
            self.popup.handle_key_event(kv);
            return;
        }
        let scope = self.tabs[self.tab_index as usize].input_scope();
        let map_scope = if self.help.is_active() {
            self.input.cancel();
            if !self.input.starts_chord(key::Scope::App, kv) {
                self.help.handle_key_event(kv);
                return;
            }
            self.help.dismiss();
            key::Scope::App
        } else {
            scope
        };
        if let key::Match::Execute(action) = self.input.feed(kv, scope, map_scope) {
            match action {
                key::Action::App(action) => self.handle_action(action),
                action => {
                    if let key::Action::Logs(action) = action {
                        use tab::logs::Action::*;
                        if matches!(
                            action,
                            TogglePause
                                | FzfFind
                                | Clear
                                | ToggleDebug
                                | ToggleInfo
                                | ToggleWarning
                                | ToggleError
                        ) {
                            self.log_refresh_at = None;
                        }
                    }
                    self.with_tab(self.tab_index as usize, |tab, business| {
                        tab.handle_action(action, business)
                    });
                }
            }
            self.sync_visible();
        }
    }
    fn render(&mut self, f: &mut ratatui::Frame) {
        use ratatui::prelude::{Constraint, Layout};

        let chunks = Layout::default()
            .constraints([Constraint::Length(3), Constraint::Fill(1)])
            .split(f.area());

        let scope = self.tabs[self.tab_index as usize].input_scope();
        let maps = key::keymaps();
        let hints = self.input.hints(scope);
        render_tabbar(
            tab_titles(self.tabs.iter().map(|tab| tab.title()), maps),
            self.tab_index,
            f,
            chunks[0],
        );

        self.tabs[self.tab_index as usize].render(f, chunks[1], self.business.view());

        if let Some(hints) = hints {
            widget::which::render(f, &hints);
        }
        if self.help.is_active() {
            self.help
                .render(f, self.tabs[self.tab_index as usize].title(), maps, scope);
        }

        if self.popup.check() {
            self.popup.render(f, Default::default());
        }
    }

    fn switch_tab(&mut self, new_index: u8) {
        if new_index != self.tab_index {
            self.input.cancel();
            let previous = self.tab_index as usize;
            self.with_tab(previous, |tab, cx| tab.on_leave(cx));
            self.log_refresh_at = None;
            self.tab_index = new_index;
            self.with_tab(new_index as usize, |tab, cx| tab.on_enter(cx));
        }
    }

    fn handle_action(&mut self, action: AppAction) {
        match action {
            AppAction::Tab(target) => self.switch_tab(target as u8),
            AppAction::TabNext | AppAction::TabPrev => {
                let count = self.tabs.len();
                let offset = if action == AppAction::TabPrev {
                    count - 1
                } else {
                    1
                };
                self.switch_tab(((self.tab_index as usize + offset) % count) as u8);
            }
            AppAction::Quit => request_quit(),
            AppAction::RefreshCurrent => match self.tab_index {
                0 | 2 | 3 | 5 => {
                    let resource = match self.tab_index {
                        0 => ApiResource::Status,
                        2 => ApiResource::Proxies,
                        3 => ApiResource::Connections,
                        _ => ApiResource::Settings,
                    };
                    self.business.refresh(resource);
                }
                6 => self.business.check_services(),
                4 => self.business.confirm_api(),
                1 => {
                    tab::files::request_refresh();
                }
                _ => {}
            },
            AppAction::Help => self.help.toggle(),
            AppAction::Global(actions::GlobalAction::RestartCoreService) => {
                self.business
                    .start_service(crate::config::CONFIG.core_type(), ServiceOperation::Restart);
            }
            AppAction::Global(actions::GlobalAction::CloseAllConnections) => {
                if self.business.view().core.api_allowed() {
                    self.business.close_all_connections();
                } else {
                    self.show_feedback(vec![
                        self.business
                            .view()
                            .core
                            .diagnostic()
                            .unwrap_or_else(|| "API unavailable".into()),
                    ]);
                }
            }
            AppAction::Global(action) => {
                self.globals.spawn_blocking(move || action.execute());
            }
        }
    }
}

fn tab_titles(titles: impl IntoIterator<Item = &'static str>, maps: &key::Keymaps) -> Vec<String> {
    TabTarget::ALL
        .into_iter()
        .zip(titles)
        .map(|(target, title)| {
            let keys: Vec<_> = maps
                .tab_bindings(target)
                .iter()
                .map(|binding| binding.key_text.as_str())
                .collect();
            if keys.is_empty() {
                title.to_owned()
            } else {
                format!("{} {title}", keys.join("/"))
            }
        })
        .collect()
}

/// each item should represent for one tab
fn render_tabbar(
    titles: impl IntoIterator<Item = String>,
    selected: u8,
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
) {
    use crate::tui::theme::Theme;
    use ratatui::style::Styled;
    use ratatui::widgets::{Block, Tabs};

    let theme = Theme::get();
    let block = Block::bordered().title(" Clashtui ");
    let titles = titles
        .into_iter()
        .map(|title| title.set_style(theme.tabbar.text));
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
mod tests;

#[cfg(test)]
mod log_refresh_tests {
    use super::*;

    #[test]
    fn continuous_notifications_and_other_wakes_do_not_postpone_the_deadline() {
        let mut refresh_at = None;
        let start = Instant::now();
        for frame in 0..3 {
            let first = start + LOG_REFRESH_INTERVAL * frame;
            let deadline = first + LOG_REFRESH_INTERVAL;
            for elapsed in 0..LOG_REFRESH_INTERVAL.as_millis() as u64 {
                let now = first + Duration::from_millis(elapsed);
                schedule_log_refresh(true, &mut refresh_at, now);
                // This check also runs after input/task wakes, before drawing or waiting.
                assert!(!take_log_refresh(true, &mut refresh_at, now));
                assert_eq!(refresh_at, Some(deadline));
            }
            assert!(take_log_refresh(true, &mut refresh_at, deadline));
            assert!(refresh_at.is_none());
        }
        assert!(!take_log_refresh(
            true,
            &mut refresh_at,
            start + Duration::from_secs(3600)
        ));
    }

    #[test]
    fn hidden_views_and_local_snapshots_cancel_the_previous_display_deadline() {
        let mut refresh_at = None;
        let now = Instant::now();
        for _ in 0..1000 {
            schedule_log_refresh(false, &mut refresh_at, now);
        }
        assert!(refresh_at.is_none());
        schedule_log_refresh(true, &mut refresh_at, now);
        assert!(!take_log_refresh(false, &mut refresh_at, now));
        assert!(refresh_at.is_none());
        schedule_log_refresh(true, &mut refresh_at, now);
        refresh_at = None; // Enter, pause, clear, or level control already copied the snapshot.
        assert!(!take_log_refresh(
            true,
            &mut refresh_at,
            now + LOG_REFRESH_INTERVAL
        ));
    }
}
