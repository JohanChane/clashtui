// use std::cell::OnceCell;
mod bars;
mod consts;
mod key_bind;
pub mod tabs;

use super::widget::{ConfirmPopup, ListPopup};
use super::{Call, Drawable, EventState, Theme};
use crate::utils::{consts::err as consts_err, CallBack};
use key_bind::Keys;
use tabs::{TabCont, TabContainer};

use crossterm::event::{self, KeyCode, KeyEvent};
use ratatui::prelude as Ra;
use ratatui::widgets as Raw;
use tokio::sync::mpsc::{Receiver, Sender};
use Ra::{Frame, Rect};

pub struct FrontEnd {
    tabs: Vec<TabContainer>,
    tab_index: usize,
    there_is_list_pop: bool,
    list_popup: Option<Box<ListPopup>>,
    there_is_msg_pop: bool,
    msg_popup: ConfirmPopup,
    should_quit: bool,
    state: Option<String>,
    backend_content: Option<Call>,
}

impl FrontEnd {
    pub fn new() -> Self {
        let service_tab = tabs::service::ServiceTab::new();
        let profile_tab = tabs::profile::ProfileTab::new();
        let tabs = vec![profile_tab.into(), service_tab.into()];
        Self {
            tabs,
            tab_index: 0,
            there_is_list_pop: false,
            list_popup: Default::default(),
            there_is_msg_pop: false,
            msg_popup: ConfirmPopup::new(),
            should_quit: false,
            state: None,
            backend_content: None,
        }
    }
    pub async fn run(mut self, tx: Sender<Call>, mut rx: Receiver<CallBack>) -> anyhow::Result<()> {
        use core::time::Duration;
        use futures::StreamExt as _;
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
            let ev;
            tokio::select! {
                // avoid tokio cancel the handler
                e = stream.next() => {ev = e},
                // make sure tui refresh
                () = tokio::time::sleep(TICK_RATE) => {ev = None}
            };
            if let Some(ev) = ev {
                if let event::Event::Key(key) = ev? {
                    self.handle_key_event(&key);
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
        let msg = self.tabs[self.tab_index].get_popup_content();
        if let Some(msg) = msg {
            self.msg_popup.show_msg(msg);
            self.there_is_msg_pop = true;
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
                        self.msg_popup.clear();
                        self.msg_popup.show_msg(super::PopMsg::Prompt(vec![
                            "Error Happened".to_owned(),
                            error,
                        ]));
                        self.there_is_msg_pop = true;
                    }
                    CallBack::Logs(logs) => {
                        self.get_list_popup().set("Log", logs);
                        self.there_is_list_pop = true;
                    }
                    CallBack::Infos(infos) => {
                        self.get_list_popup().set("Infos", infos);
                        self.there_is_list_pop = true;
                    }
                    // `SwitchMode` goes here
                    // Just update StateBar
                    CallBack::State(state) => {
                        self.state.replace(state);
                    }
                    // assume ProfileTab is the first tab
                    CallBack::ProfileInit(..) => self.tabs[0].apply_backend_call(op),
                    #[cfg(feature = "template")]
                    CallBack::TemplateInit(_) => self.tabs[0].apply_backend_call(op),
                    CallBack::ProfileCTL(_) | CallBack::ServiceCTL(_) => {
                        self.tabs[self.tab_index].apply_backend_call(op)
                    }
                },
                Err(e) => match e {
                    tokio::sync::mpsc::error::TryRecvError::Empty => break,
                    tokio::sync::mpsc::error::TryRecvError::Disconnected => {
                        unreachable!("{}", consts_err::BACKEND_TX);
                    }
                },
            }
        }
    }
    fn get_list_popup(&mut self) -> &mut Box<ListPopup> {
        if let None = self.list_popup.as_ref() {
            self.list_popup = Some(Box::new(ListPopup::new()));
        }
        self.list_popup.as_mut().unwrap()
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
        if self.there_is_list_pop {
            self.get_list_popup().render(f, chunks[1], true)
        }
        if self.there_is_msg_pop {
            self.msg_popup.render(f, chunks[1], true)
        }
    }

    fn handle_key_event(&mut self, ev: &KeyEvent) -> EventState {
        let mut evst;
        let tab = self.tabs.get_mut(self.tab_index).unwrap();
        // handle popups first
        // handle them only in app to avoid complexity.
        //
        // if there is a popup, other part will be blocked.
        if self.there_is_msg_pop {
            evst = self.msg_popup.handle_key_event(ev);
            if let EventState::WorkDone = tab.apply_popup_result(evst) {
                self.there_is_msg_pop = false;
            }
            return EventState::WorkDone;
        }
        if self.there_is_list_pop {
            evst = self.get_list_popup().handle_key_event(ev);
            if let EventState::Cancel = evst {
                self.there_is_list_pop = false;
            }
            return EventState::WorkDone;
        }
        // handle tabs second
        // select the very one rather iter over vec
        evst = tab.handle_key_event(ev);

        // handle tabbar and app owned last
        if evst == EventState::NotConsumed {
            if ev.kind != crossterm::event::KeyEventKind::Press {
                return EventState::NotConsumed;
            }
            // if work is not done, this will be reset
            evst = EventState::WorkDone;
            match ev.code {
                // ## the tabbar
                // 1..=9
                // need to kown the range
                #[allow(clippy::is_digit_ascii_radix)]
                KeyCode::Char(c) if c.is_digit(10) && c != '0' => {
                    let digit = c.to_digit(10);
                    if let Some(d) = digit {
                        if d <= self.tabs.len() as u32 {
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
                Keys::LogCat => self.backend_content = Some(Call::Logs(0, 20)),
                Keys::AppHelp => {
                    self.get_list_popup().set(
                        "Help",
                        Keys::const_doc()
                            .into_iter()
                            .map(|s| s.to_owned())
                            .collect(),
                    );
                    self.there_is_list_pop = true;
                }
                Keys::AppInfo => self.backend_content = Some(Call::Infos),
                Keys::AppQuit => {
                    self.should_quit = true;
                }
                _ => evst = EventState::NotConsumed,
            }
        }
        evst
    }
}
