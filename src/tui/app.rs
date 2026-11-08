mod actions;
mod hub;

use hub::Hub;

use super::*;
pub(in crate::tui) use actions::{AppAction, TabTarget, km};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use tab::TuiTab;
use widget::help::HelpPanel;
use widget::popmsg::PopUp;

// 50fps
const TICK_RATE: std::time::Duration = std::time::Duration::from_millis(20);
pub(super) static SPINNER_FRAME: AtomicU8 = AtomicU8::new(0);
pub(crate) static QUIT: AtomicBool = AtomicBool::new(false);

pub struct App {
    hub: Hub,
    popup: PopUp,
    help: HelpPanel,
    input: key::Matcher<'static>,
}

impl App {
    fn new() -> Self {
        Self {
            hub: Hub::new(),
            popup: PopUp::default(),
            help: HelpPanel::default(),
            input: key::Matcher::new(key::keymaps()),
        }
    }
    #[tokio::main]
    pub async fn serve() -> anyhow::Result<()> {
        let mut coordinator = term::TerminalCoordinator::install();
        let signals = signals::Signals::start()?;
        // Open before constructing Hub: CoreMonitor may emit as soon as it starts.
        let mut inbox = event::Inbox::open();
        let mut app = Self::new();
        app.hub.start();
        let mut input = term::InputReader::start();
        let mut invt = tokio::time::interval(TICK_RATE);
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(std::io::stdout()))?;
        let mut pause_ack: Option<tokio::sync::oneshot::Receiver<()>> = None;
        let mut pending_control = None;
        let mut quitting = false;

        loop {
            if QUIT.load(Ordering::Relaxed) && !quitting {
                quitting = true;
                signals.close();
                input.begin_shutdown();
                coordinator.begin_shutdown();
            }
            if quitting && coordinator.is_idle() && !input.has_worker() && pending_control.is_none()
            {
                break;
            }
            if !quitting && coordinator.owns_terminal() && pause_ack.is_none() {
                terminal.draw(|f| app.render(f))?;
                app.sync();
            }

            tokio::select! {
                event = async {
                    if quitting {
                        std::future::poll_fn(|cx| input.poll_finished(cx)).await;
                        None
                    } else {
                        input.next_event().await
                    }
                }, if (quitting && input.has_worker()) || (!quitting && coordinator.owns_terminal() && pause_ack.is_none()) => {
                    let Some(event) = event else {
                        QUIT.store(true, Ordering::Relaxed);
                        continue;
                    };
                    use crossterm::event::Event;
                    match event? {
                        Event::Key(key_event) if key_event.kind.is_press() => {
                            #[cfg(debug_assertions)]
                            the_egg(key_event.code);
                            let key = key_event.into();
                            app.handle_key_event(&key);
                        }
                        Event::Resize(..) => terminal.autoresize()?,
                        _ => {}
                    }
                }
                // if we switch between screens
                control = coordinator.recv() => match control {
                    Some(control) if control.is_suspend() && !quitting => {
                        // first we hold tui for output
                        pause_ack = Some(input.request_pause());
                        pending_control = Some(control);
                    }
                    Some(control) => {
                        let clear = coordinator.handle(control)?;
                        if !quitting && coordinator.owns_terminal() {
                            input.resume();
                            if clear {
                                // then we tell ratatui to re-render everything
                                terminal.clear()?;
                            }
                        }
                    }
                    None => break,
                },
                result = async { pause_ack.as_mut().unwrap().await }, if pause_ack.is_some() => {
                    pause_ack = None;
                    if result.is_err() && !quitting {
                        anyhow::bail!("Terminal input stopped before confirming suspension");
                    }
                    if let Some(control) = pending_control.take() {
                        let clear = coordinator.handle(control)?;
                        if !quitting && coordinator.owns_terminal() {
                            input.resume();
                            if clear {
                                terminal.clear()?;
                            }
                        }
                    }
                }
                // Keep events queued while the terminal is lent out.
                Some(event) = inbox.recv(), if !quitting && coordinator.owns_terminal() && pause_ack.is_none() => app.on_event(event),
                // this tick here ensures that fps is stable
                // Keep polling QUIT even while a worker owns the terminal.
                _ = invt.tick() => {}
            }
        }

        signals.shutdown().await;
        log::trace!("App Exit");
        Ok(())
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
                action => self.hub.shown().handle_action(action),
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
        render_tabbar(tab_titles(self.hub.titles(), maps), shown, f, chunks[0]);

        self.hub.shown().render(f, chunks[1]);

        if let Some(hints) = hints {
            widget::which::render(f, &hints);
        }
        if self.help.is_active() {
            self.help.render(f, self.hub.shown().title(), maps, scope);
        }

        if self.popup.check() {
            self.popup.render(f, Default::default());
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
        if let event::Event::Core(event::CoreEvent::Switched(_)) = event {
            // Key bindings and pages depend on the core type; switching cores requires restarting clashtui.
            QUIT.store(true, Ordering::Relaxed);
        }
        self.hub.dispatch(&event);
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
                tokio::task::spawn_blocking(move || {
                    if let Err(error) = action.execute() {
                        popmsg::prelude::Confirm::err(format!("{error:#}"));
                    }
                });
            }
        }
    }
    fn sync(&mut self) {
        SPINNER_FRAME.fetch_add(1, Ordering::Relaxed);
        self.popup.sync();
        if self.popup.check() {
            self.input.cancel();
        }
        self.hub.sync();
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
