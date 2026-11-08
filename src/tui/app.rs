mod actions;
mod hub;

use hub::Hub;

use super::*;
#[cfg(test)]
pub(in crate::tui) use actions::TabTarget;
pub(in crate::tui) use actions::{AppAction, km};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use tab::TuiTab;
use widget::help::HelpPanel;
use widget::popmsg::PopUp;

// 50fps
const TICK_RATE: std::time::Duration = std::time::Duration::from_millis(20);
pub(super) static FULL_RENDER: tokio::sync::Notify = tokio::sync::Notify::const_new();
pub(super) static SPINNER_FRAME: AtomicU8 = AtomicU8::new(0);
pub(crate) static QUIT: AtomicBool = AtomicBool::new(false);

pub struct App {
    hub: Hub,
    commands: commands::Commands,
    status_due: [std::time::Instant; 2],
    next_status: usize,
    copy_notice: Option<(&'static str, std::time::Instant)>,
    popup: PopUp,
    help: HelpPanel,
    input: key::Matcher<'static>,
    switched_core: Option<crate::config::CoreType>,
}

impl App {
    fn new() -> Self {
        Self {
            hub: Hub::new(),
            commands: commands::Commands::default(),
            status_due: [std::time::Instant::now(); 2],
            next_status: crate::config::CONFIG.core_type().index(),
            copy_notice: None,
            popup: PopUp::default(),
            help: HelpPanel::default(),
            input: key::Matcher::new(key::keymaps()),
            switched_core: None,
        }
    }
    #[tokio::main]
    pub async fn serve() -> anyhow::Result<()> {
        let mut inbox = event::Inbox::open();
        let mut app = Self::new();
        app.hub.start();
        let cancel = app.commands.cancellation();
        let mut signals = signals::Signals::start(cancel.clone())?;
        let mut coordinator = term::Coordinator::register()?;
        let mut events = input::InputReader::start();
        let mut invt = tokio::time::interval(TICK_RATE);
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(std::io::stdout()))?;

        let result = async {
            while !QUIT.load(Ordering::Relaxed) && !crate::command::shutdown_requested() {
                app.sync();
                terminal.draw(|f| app.render(f))?;

                let ev = {
                    // this tick here ensures that fps is stable
                    let mut tick = Box::pin(invt.tick());
                    let ev = tokio::select! {
                        ev = events.next_event() => match ev {
                            Some(ev) => ev?,
                            None => anyhow::bail!("Terminal input reader stopped"),
                        },
                        _ = &mut tick => continue,
                        Some(event) = inbox.recv() => { app.on_event(event); continue; },
                        Some(request) = coordinator.next() => {
                            coordinator.handoff(request, &mut events, &cancel).await?;
                            terminal.clear()?;
                            continue;
                        },
                        // if we switch between screens
                        _ = FULL_RENDER.notified() => {
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

            Ok::<(), anyhow::Error>(())
        }
        .await;
        events.begin_shutdown();
        app.commands.begin_shutdown();
        coordinator.close();
        while !app.commands.is_drained() {
            if let Some(event) = app.commands.poll() {
                app.commands.acknowledge(event.task());
            }
            tokio::time::sleep(TICK_RATE).await;
        }
        if events.has_worker() {
            std::future::poll_fn(|cx| events.poll_finished(cx)).await;
        }
        signals.shutdown().await;
        log::trace!("App Exit");
        result
    }

    /// Popups capture text input; all page/global shortcuts share one compiled map.
    /// Help can start a global-only chord, which retains its candidates until completion.
    fn handle_key_event(&mut self, kv: &Key) {
        log::debug!("K: {kv:?}");
        // This final notice owns every key, including Tab, Esc and Ctrl-C.
        if self.switched_core.is_some() {
            QUIT.store(true, Ordering::Relaxed);
            return;
        }
        if kv.code == crossterm::event::KeyCode::Char('c')
            && kv
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL)
        {
            self.commands.cancel();
            return;
        }
        if self.popup.check() {
            self.input.cancel();
            self.popup.handle_key_event(kv);
            return;
        }
        let scope = self.hub.shown().input_scope();
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
                action => self.hub.shown().handle_action(action, &mut self.commands),
            }
        }
    }
    fn render(&mut self, f: &mut ratatui::Frame) {
        use ratatui::prelude::{Constraint, Layout};

        let chunks = Layout::default()
            .constraints([Constraint::Length(3), Constraint::Fill(1)])
            .split(f.area());

        let scope = self.hub.shown().input_scope();
        let maps = key::keymaps();
        let hints = self.input.hints(scope);
        let shown = self.hub.shown_index() as u8;
        render_tabbar(tab_titles(self.hub.titles()), shown, f, chunks[0]);

        self.hub.shown().render(f, chunks[1]);

        if let Some((message, until)) = self.copy_notice
            && until > std::time::Instant::now()
        {
            let area = f.area();
            let notice =
                ratatui::layout::Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1);
            f.render_widget(ratatui::widgets::Clear, notice);
            f.render_widget(ratatui::widgets::Paragraph::new(message), notice);
        }

        if let Some(hints) = hints {
            widget::which::render(f, &hints);
        }
        if self.help.is_active() {
            self.help.render(f, self.hub.shown().title(), maps, scope);
        }

        if self.popup.check() {
            self.popup.render(f, Default::default());
        }
        if let Some(core) = self.switched_core {
            render_core_switched(core, f);
        }
    }

    fn switch_tab(&mut self, new_index: u8) {
        if new_index as usize == self.hub.shown_index() {
            return;
        }
        self.input.cancel();
        self.hub.show(new_index as usize);
    }

    fn on_event(&mut self, event: event::Event) {
        if self.switched_core.is_none() {
            self.hub.dispatch(&event);
        }
    }

    fn handle_action(&mut self, action: AppAction) {
        match action {
            AppAction::Tab(target) => self.switch_tab(target as u8),
            AppAction::TabNext | AppAction::TabPrev => {
                let count = self.hub.len();
                let offset = if action == AppAction::TabPrev {
                    count - 1
                } else {
                    1
                };
                self.switch_tab(((self.hub.shown_index() + offset) % count) as u8);
            }
            AppAction::Quit => {
                QUIT.store(true, Ordering::Relaxed);
            }
            AppAction::Help => self.help.toggle(),
            AppAction::Global(action) => {
                if let Err(error) = action.execute(&mut self.commands) {
                    popmsg::prelude::Confirm::err(format!("{error:#}"));
                }
            }
        }
    }
    fn sync(&mut self) {
        if self.switched_core.is_some() {
            return;
        }
        SPINNER_FRAME.fetch_add(1, Ordering::Relaxed);
        while let Some(event) = self.commands.poll() {
            use commands::CommandEvent;
            self.hub.command_done(&event);
            match &event {
                CommandEvent::CoreSwitched {
                    src, dst, result, ..
                } => self.core_switch_done(*src, *dst, result),
                CommandEvent::Script { target, result, .. } => {
                    if target.changes_service {
                        self.status_due[target.core.index()] = std::time::Instant::now();
                        monitor::set_service_status(target.core, None);
                        event::emit(event::CoreEvent::ServiceCommandDone);
                    }
                    match commands::present(target, result) {
                        Ok(Some(message)) => {
                            self.copy_notice = Some((
                                message,
                                std::time::Instant::now() + std::time::Duration::from_secs(3),
                            ))
                        }
                        Ok(None) => (),
                        Err(error) => popmsg::prelude::Confirm::err(format!("{error:#}")),
                    }
                }
                CommandEvent::Tested { target, result, .. } => match result {
                    Ok(text) => {
                        popmsg::prelude::Confirm::title(format!("Test: {}", target.name))
                            .with_prompt(text.clone())
                            .build_and_send();
                    }
                    Err(error) => popmsg::prelude::Confirm::err(commands::error_text(error)),
                },
                CommandEvent::Checked {
                    target,
                    purpose,
                    result,
                    ..
                } => match result {
                    Ok(()) if *purpose == commands::CheckPurpose::Inspect => {
                        popmsg::prelude::Confirm::title("Check Passed".into())
                            .with_prompt(target.name.clone())
                            .build_and_send();
                    }
                    Ok(()) => (),
                    Err(error) => popmsg::prelude::Confirm::err(commands::error_text(error)),
                },
                CommandEvent::Failed { error, .. } => popmsg::prelude::Confirm::err(error.clone()),
                CommandEvent::Status { core, result, .. } => {
                    match result {
                        Ok(report) => {
                            if let Some(diagnostic) = &report.diagnostic {
                                log::warn!("{core}: status_core: {diagnostic}");
                            }
                        }
                        Err(error) => {
                            log::warn!("{core}: status_core: {}", commands::error_text(error));
                        }
                    }
                    monitor::set_service_status(
                        *core,
                        result.as_ref().ok().map(|report| report.state),
                    );
                }
            }
            self.commands.acknowledge(event.task());
        }
        if self.switched_core.is_some() {
            return;
        }
        if !self.commands.is_busy() {
            let now = std::time::Instant::now();
            if let Some(core) = (0..2)
                .map(|offset| crate::config::CoreType::ALL[(self.next_status + offset) % 2])
                .find(|core| self.status_due[core.index()] <= now)
            {
                self.next_status = (core.index() + 1) % 2;
                self.status_due[core.index()] = now + std::time::Duration::from_secs(5);
                if let Err(error) = self
                    .commands
                    .submit(core, crate::scripts::ActionRequest::Status)
                {
                    log::warn!("{error:#}");
                }
            }
        }
        self.popup.sync();
        if self.popup.check() {
            self.input.cancel();
        }
        self.hub.sync();
    }

    fn core_switch_done(
        &mut self,
        src: crate::config::CoreType,
        dst: crate::config::CoreType,
        result: &anyhow::Result<()>,
    ) {
        self.status_due[src.index()] = std::time::Instant::now();
        monitor::set_service_status(src, None);
        event::emit(event::CoreEvent::ServiceCommandDone);
        match result {
            Ok(()) => {
                self.switched_core = Some(dst);
                self.input.cancel();
                self.commands.begin_shutdown();
            }
            Err(error) => popmsg::prelude::Confirm::err(format!(
                "Failed to switch core to {dst}: {}",
                commands::error_text(error)
            )),
        }
    }
}

fn render_core_switched(core: crate::config::CoreType, f: &mut ratatui::Frame) {
    use ratatui::{
        layout::{Constraint, Layout},
        widgets::{Block, Clear, Paragraph, Wrap},
    };
    let [_, row, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(7),
        Constraint::Fill(1),
    ])
    .areas(f.area());
    let [_, area, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(54),
        Constraint::Fill(1),
    ])
    .areas(row);
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(format!(
            "Switched to {core}.\n\nPress any key to exit clashtui.\nOpen clashtui again to use {core}."
        ))
        .centered()
        .wrap(Wrap { trim: false })
        .block(Block::bordered().title("Core Switched")
            .border_style(theme::Theme::get().popup.border)),
        area,
    );
}

fn tab_titles(titles: impl IntoIterator<Item = &'static str>) -> Vec<String> {
    titles
        .into_iter()
        .enumerate()
        .map(|(index, title)| format!("{} {title}", index + 1))
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
