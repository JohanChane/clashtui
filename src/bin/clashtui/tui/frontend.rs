// use std::cell::OnceCell;
mod bars;
mod consts;
mod key_bind;
pub mod tabs;

use super::widget::ConfirmPopup;
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
    // help_popup: OnceCell<Box<HelpPopUp>>,
    // info_popup: Box<InfoPopUp>,
    there_is_pop: bool,
    global_popup: ConfirmPopup,
    should_quit: bool,
    state: Option<String>,
}

impl FrontEnd {
    pub fn new() -> Self {
        let service_tab = tabs::service::ServiceTab::new();
        let tabs = vec![service_tab.into()];
        Self {
            tabs,
            tab_index: 0,
            // help_popup: Default::default(),
            there_is_pop: false,
            global_popup: ConfirmPopup::new(),
            should_quit: false,
            state: None,
        }
    }
    pub async fn run(mut self, tx: Sender<Call>, mut rx: Receiver<CallBack>) -> anyhow::Result<()> {
        use core::time::Duration;
        use futures::StreamExt as _;
        const TICK_RATE: Duration = Duration::from_millis(250);
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
            tokio::select! {
                Some(ev) = stream.next() => {
                    if let event::Event::Key(key) = ev?{
                        self.handle_key_event(&key);
                    }
                },
                // make sure tui refresh
                () = tokio::time::sleep(TICK_RATE) => {}
            };
        }
        tx.send(Call::Stop).await.expect(consts_err::BACKEND_TX);
        log::info!("App Exit");
        Ok(())
    }
    async fn tick(&mut self, tx: &Sender<Call>, rx: &mut Receiver<CallBack>) {
        // the backend thread is activated by this call
        tx.send(Call::Tick).await.expect(consts_err::BACKEND_TX);
        let msg = self.tabs[self.tab_index].get_popup_content();
        if let Some(msg) = msg {
            self.global_popup.show_msg(msg);
            self.there_is_pop = true;
        }
        let op = self.tabs[self.tab_index].get_backend_call();
        if let Some(op) = op {
            tx.send(op).await.expect(consts_err::BACKEND_RX);
        }
        let op = rx.try_recv();
        match op {
            Ok(op) => match op {
                CallBack::Error(error) => {
                    self.global_popup.clear();
                    self.global_popup.show_msg(super::PopMsg::Notice(vec![
                        "Error Happened".to_owned(),
                        error,
                    ]));
                    self.there_is_pop = true;
                }
                // `SwitchMode` goes here
                // Just update StateBar
                CallBack::State(state) => {
                    self.state.replace(state);
                }
                CallBack::ServiceCTL(_) => self.tabs[self.tab_index].apply_backend_call(op),
            },
            Err(e) => match e {
                tokio::sync::mpsc::error::TryRecvError::Empty => (),
                tokio::sync::mpsc::error::TryRecvError::Disconnected => {
                    unreachable!("{}", consts_err::BACKEND_TX);
                }
            },
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
        if self.there_is_pop {
            self.global_popup.render(f, chunks[1], true)
        }
    }

    fn handle_key_event(&mut self, ev: &KeyEvent) -> EventState {
        let mut evst;
        let tab = self.tabs.get_mut(self.tab_index).unwrap();
        // handle popups first
        // handle them only in app to avoid complexity
        if self.there_is_pop {
            evst = self.global_popup.handle_key_event(ev);
            if let EventState::WorkDone = tab.apply_popup_result(evst) {
                self.there_is_pop = false;
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
                Keys::AppHelp => todo!(),
                Keys::AppInfo => todo!(),
                Keys::AppQuit => {
                    self.should_quit = true;
                }
                _ => evst = EventState::NotConsumed,
            }
        }
        evst
    }
}
