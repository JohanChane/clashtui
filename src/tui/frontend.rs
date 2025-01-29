mod bars;
mod consts;
mod key_bind;
pub mod tabs;

use super::widget::Popup;
use super::{Call, Drawable, EventState, Theme};
use crate::backend::CallBack;
use crate::utils::consts::err as consts_err;
use key_bind::Keys;
use tabs::TabCont;

use crossterm::event::{self, KeyCode, KeyEvent};
use ratatui::prelude as Ra;
use ratatui::widgets as Raw;
use tokio::sync::mpsc::{Receiver, Sender};
use Ra::{Frame, Rect};

pub struct FrontEnd {
    tabs: Vec<Box<dyn TabCont + Send>>,
    tab_index: usize,
    popup: Box<Popup>,
    should_quit: bool,
    /// StateBar
    state: Option<String>,
    backend_content: Option<Call>,
}

impl FrontEnd {
    pub fn new() -> Self {
        let tabs: Vec<Box<dyn TabCont + Send>> = vec![
            Box::new(tabs::profile::ProfileTab::default()),
            Box::new(tabs::service::ServiceTab::default()),
            #[cfg(feature = "connection-tab")]
            Box::new(tabs::connection::ConnctionTab::default()),
        ];
        Self {
            tabs,
            tab_index: 0,
            popup: Default::default(),
            should_quit: false,
            state: None,
            backend_content: None,
        }
    }
    pub async fn run(mut self, tx: Sender<Call>, mut rx: Receiver<CallBack>) -> anyhow::Result<()> {
        use core::time::Duration;
        use futures_util::StreamExt as _;
        // 50fps
        const TICK_RATE: Duration = Duration::from_millis(20);
        let mut terminal = Ra::Terminal::new(Ra::CrosstermBackend::new(std::io::stdout()))?;
        // this is an async solution.
        // NOTE:
        // `event::poll` will block the thread,
        // and since there is only one process,
        // the backend thread won't be able to active
        let mut stream = event::EventStream::new();
        while !self.should_quit {
            self.tick(&tx, &mut rx).await;
            terminal.draw(|f| self.render(f, Default::default(), true))?;
            let ev = tokio::select! {
                // avoid tokio cancel the handler
                e = stream.next() => {e},
                // make sure tui refresh
                () = tokio::time::sleep(TICK_RATE) => {None}
            };
            if let Some(ev) = ev {
                match ev? {
                    event::Event::FocusGained | event::Event::FocusLost => (),
                    event::Event::Mouse(_) => (),
                    event::Event::Paste(_) => (),
                    event::Event::Key(key_event) => {
                        #[cfg(debug_assertions)]
                        if the_egg(key_event.code) {
                            log::debug!("You've found the egg!")
                        };
                        self.handle_key_event(&key_event);
                    }
                    event::Event::Resize(..) => terminal.autoresize()?,
                }
            }
        }
        tx.send(Call::Stop).await.expect(consts_err::BACKEND_TX);
        log::info!("App Exit");
        Ok(())
    }
    async fn tick(&mut self, tx: &Sender<Call>, rx: &mut Receiver<CallBack>) {
        // the backend thread is activated by this call
        tx.send(Call::Tick).await.expect(consts_err::BACKEND_TX);
        // handle tab msg
        if let Some(msg) = self.tabs[self.tab_index].get_popup_content() {
            self.popup.show_msg(msg);
        }
        // handle app ops
        if let Some(op) = self.backend_content.take() {
            tx.send(op).await.expect(consts_err::BACKEND_RX);
        }
        // handle tab ops
        let op = self.tabs[self.tab_index].get_backend_call();
        if let Some(op) = op {
            tx.send(op).await.expect(consts_err::BACKEND_RX);
        }
        // try to handle as much to avoid channel overflow
        loop {
            let op = rx.try_recv();
            match op {
                Ok(op) => match op {
                    CallBack::Error(error) => {
                        self.popup.show_msg(super::PopMsg::Prompt(vec![
                            "Error Happened".to_owned(),
                            error,
                        ]));
                    }
                    CallBack::Logs(logs) => {
                        self.popup.set_msg("Log", logs);
                    }
                    CallBack::Infos(infos) => {
                        self.popup.set_msg("Infos", infos);
                    }
                    CallBack::Preview(content) => {
                        self.popup.set_msg("Preview", content);
                    }
                    CallBack::Edit => {
                        self.popup
                            .show_msg(super::PopMsg::Prompt(vec!["OK".to_owned()]));
                    }
                    // `SwitchMode` goes here
                    // Just update StateBar
                    CallBack::State(state) => {
                        self.state.replace(state);
                    }
                    // assume ProfileTab is the first tab
                    CallBack::ProfileInit(..) => self.tabs[0].apply_backend_call(op),
                    CallBack::ProfileCTL(_) | CallBack::ServiceCTL(_) => {
                        self.tabs[self.tab_index].apply_backend_call(op)
                    }
                    #[cfg(feature = "template")]
                    CallBack::TemplateInit(_) => self.tabs[0].apply_backend_call(op),
                    #[cfg(feature = "template")]
                    CallBack::TemplateCTL(_) => self.tabs[self.tab_index].apply_backend_call(op),
                    // assume ConnctionTab is the third tab
                    #[cfg(feature = "connection-tab")]
                    CallBack::ConnctionInit(..) => self.tabs[2].apply_backend_call(op),
                    #[cfg(feature = "connection-tab")]
                    CallBack::ConnctionCTL(_) => self.tabs[self.tab_index].apply_backend_call(op),
                },
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    unreachable!("{}", consts_err::BACKEND_TX);
                }
            }
        }
    }
}

impl Drawable for FrontEnd {
    fn render(&mut self, f: &mut Frame, _: Rect, _: bool) {
        // split terminal into three part
        let chunks = Ra::Layout::default()
            .constraints(
                [
                    Ra::Constraint::Length(3),
                    Ra::Constraint::Min(0),
                    Ra::Constraint::Length(3),
                ]
                .as_ref(),
            )
            .split(f.area());
        self.render_tabbar(f, chunks[0]);
        self.render_statusbar(f, chunks[2]);
        self.tabs
            .get_mut(self.tab_index)
            .unwrap()
            .render(f, chunks[1], true);
        if !self.popup.is_empty() {
            self.popup.render(f, chunks[1], true)
        }
    }

    fn handle_key_event(&mut self, ev: &KeyEvent) -> EventState {
        let evst;
        // handle popups first
        // handle them only in app to avoid complexity.
        //
        // if there is a popup, other part will be blocked.
        if !self.popup.is_empty() {
            evst = self.popup.handle_key_event(ev);
            if evst == EventState::Yes || evst == EventState::Cancel {
                if let Some(res) = self.popup.collect() {
                    self.tabs
                        .get_mut(self.tab_index)
                        .unwrap()
                        .apply_popup_result(res);
                }
            } else if evst == EventState::Cancel {
                self.popup.clear();
            }
            return EventState::WorkDone;
        }
        let tab = self.tabs.get_mut(self.tab_index).unwrap();
        // handle tabs second
        // select the very one rather iter over vec
        evst = tab.handle_key_event(ev);

        // handle tabbar and app owned last
        if evst.is_notconsumed() {
            if ev.kind != crossterm::event::KeyEventKind::Press {
                return EventState::NotConsumed;
            }
            match ev.code {
                // ## the tabbar
                // 1..=9
                // need to kown the range
                KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                    if let Some(d) = c.to_digit(10) {
                        if d as usize <= self.tabs.len() {
                            // select target tab
                            self.tab_index = (d - 1) as usize;
                        }
                    }
                }
                KeyCode::Tab => {
                    // select next tab
                    self.tab_index += 1;
                    if self.tab_index == self.tabs.len() {
                        // loop back
                        self.tab_index = 0;
                    }
                }
                // ## the statusbar
                _ => (),
            }
            // ## the other app function
            match ev.code.into() {
                #[cfg(debug_assertions)]
                Keys::Debug => {}
                Keys::LogCat => self.backend_content = Some(Call::Logs(0, 20)),
                Keys::AppHelp => {
                    self.popup.set_msg(
                        "Help",
                        Keys::ALL_DOC.into_iter().map(|s| s.to_owned()).collect(),
                    );
                }
                Keys::AppInfo => self.backend_content = Some(Call::Infos),
                Keys::AppQuit => {
                    self.should_quit = true;
                }
                _ => return EventState::NotConsumed,
            }
        }
        EventState::WorkDone
    }
}

#[cfg(debug_assertions)]
fn the_egg(key: KeyCode) -> bool {
    static INSTANCE: std::sync::RwLock<u8> = std::sync::RwLock::new(0);
    let mut current = INSTANCE.write().unwrap();
    match *current {
        0 if key == KeyCode::Up => (),
        1 if key == KeyCode::Up => (),
        2 if key == KeyCode::Down => (),
        3 if key == KeyCode::Down => (),
        4 if key == KeyCode::Left => (),
        5 if key == KeyCode::Right => (),
        6 if key == KeyCode::Left => (),
        7 if key == KeyCode::Right => (),
        8 if (key == KeyCode::Char('b')) | (key == KeyCode::Char('B')) => (),
        9 if (key == KeyCode::Char('a')) | (key == KeyCode::Char('A')) => (),
        10 if (key == KeyCode::Char('b')) | (key == KeyCode::Char('B')) => (),
        11 if (key == KeyCode::Char('a')) | (key == KeyCode::Char('A')) => (),
        _ => {
            *current = 0;
            return false;
        }
    }
    *current += 1;
    *current == 12
}
