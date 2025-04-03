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

use Ra::{Frame, Rect};
use crossterm::event::{self, KeyCode, KeyEvent};
use ratatui::prelude as Ra;
use ratatui::widgets as Raw;
use tokio::sync::mpsc::{Receiver, Sender};

// 50fps
const TICK_RATE: std::time::Duration = std::time::Duration::from_millis(20);

pub struct FrontEnd {
    tabs: Vec<Box<dyn TabCont>>,
    tab_index: usize,
    popup: Box<Popup>,
    should_quit: bool,
    /// StateBar
    state: String,
    backend_content: Option<Call>,
}

impl FrontEnd {
    pub fn new() -> Self {
        Self {
            tabs: vec![
                tabs::service::ServiceTab::default().to_dyn(),
                tabs::profile::ProfileTab::default().to_dyn(),
                #[cfg(feature = "connections")]
                tabs::connection::ConnectionTab::default().to_dyn(),
            ],
            tab_index: 0,
            popup: Default::default(),
            should_quit: false,
            state: "Waiting State Cache Update".to_owned(),
            backend_content: None,
        }
    }
    pub async fn run(mut self, tx: Sender<Call>, mut rx: Receiver<CallBack>) -> anyhow::Result<()> {
        use futures_lite::StreamExt as _;
        let mut terminal = Ra::Terminal::new(Ra::CrosstermBackend::new(std::io::stdout()))?;
        let mut stream = event::EventStream::new();

        while !self.should_quit {
            self.tick(&tx, &mut rx).await;
            terminal.draw(|f| self.render(f, f.area(), true))?;

            let ev = tokio::select! {
                // avoid tokio cancel the handler
                Some(v) = stream.next() => {v},
                // make sure tui refresh
                () = tokio::time::sleep(TICK_RATE) => {continue},
            };
            match ev? {
                event::Event::Key(key_event) => {
                    #[cfg(debug_assertions)]
                    if the_egg(key_event.code) {
                        log::debug!("You've found the egg!")
                    };
                    self.handle_key_event(&key_event);
                }
                event::Event::Resize(..) => terminal.autoresize()?,
                _ => (),
            }
        }

        tx.send(Call::Stop).await.expect(consts_err::BACKEND_TX);
        log::trace!("App Exit");
        Ok(())
    }
    async fn tick(&mut self, tx: &Sender<Call>, rx: &mut Receiver<CallBack>) {
        // the backend thread is activated by this call
        tx.send(Call::Tick).await.expect(consts_err::BACKEND_TX);
        // handle tab msg
        if let Some(msg) = self.tabs[self.tab_index].get_popup_content() {
            self.popup.show(msg);
        }
        // handle app ops
        if let Some(op) = self.backend_content.take() {
            tx.send(op).await.expect(consts_err::BACKEND_RX);
        }
        // handle tab ops
        if let Some(op) = self.tabs[self.tab_index].get_backend_call() {
            tx.send(op).await.expect(consts_err::BACKEND_RX);
        }
        // try to handle as much to avoid channel overflow
        while let Ok(op) = rx.try_recv() {
            match op {
                CallBack::Error(error) => {
                    self.popup
                        .show(super::PopMsg::msg(format!("Error Happened\n {}", error)));
                }
                CallBack::Logs(logs) => {
                    self.popup.set_msg("Log", logs);
                }
                CallBack::Preview(content) => {
                    self.popup.set_msg("Preview", content);
                }
                CallBack::Edit => {
                    self.popup.show(super::PopMsg::msg("OK".to_owned()));
                }
                // Just update StateBar
                CallBack::State(state) => {
                    self.state = state;
                }
                CallBack::ProfileInit(..) | CallBack::ProfileCTL(_) => {
                    self.tabs[0].apply_backend_call(op)
                }
                #[cfg(feature = "template")]
                CallBack::TemplateInit(_) | CallBack::TemplateCTL(_) => {
                    self.tabs[0].apply_backend_call(op)
                }
                CallBack::TuiExtend(_) | CallBack::ServiceCTL(_) => {
                    self.tabs[1].apply_backend_call(op)
                }
                #[cfg(feature = "connections")]
                CallBack::ConnectionInit(..) | CallBack::ConnectionCTL(_) => {
                    self.tabs[2].apply_backend_call(op)
                }
            }
        }
    }
}

impl Drawable for FrontEnd {
    fn render(&mut self, f: &mut Frame, _: Rect, _: bool) {
        // split terminal into three part
        let chunks = Ra::Layout::default()
            .constraints([
                Ra::Constraint::Length(3),
                Ra::Constraint::Fill(1),
                Ra::Constraint::Length(3),
            ])
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
        let tab = self.tabs.get_mut(self.tab_index).unwrap();
        // ## Popup
        // this will block others until is done
        if !self.popup.is_empty() {
            match self.popup.handle_key_event(ev) {
                EventState::Yes => {
                    use super::widget::PopupState::{ToBackend, ToFrontend};

                    let Some(res) = self.popup.next() else {
                        return EventState::Consumed;
                    };
                    match res {
                        ToBackend(call) => self.backend_content = Some(call),
                        ToFrontend(pop_res) => tab.apply_popup_result(pop_res),
                        _ => unreachable!(),
                    }
                }
                EventState::Cancel => self.popup.reset(),
                _ => (),
            }
            return EventState::Consumed;
        }
        // ## Tabs
        if !matches!(tab.handle_key_event(ev), EventState::NotConsumed) {
            return EventState::Consumed;
        }
        // handle tabbar and app owned last
        if ev.kind != crossterm::event::KeyEventKind::Press {
            return EventState::NotConsumed;
        }
        match ev.code {
            // ## the tabbar
            // 1..=9
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
                    Keys::ALL_DOC
                        .into_iter()
                        // skip a white line
                        .skip(1)
                        .map(|s| s.to_owned())
                        .collect(),
                );
            }
            Keys::AppQuit => {
                self.should_quit = true;
            }
            _ => (),
        }

        EventState::Consumed
    }
}

#[cfg(debug_assertions)]
fn the_egg(key: KeyCode) -> bool {
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
            return false;
        }
    }
    *current += 1;
    *current == 12
}
