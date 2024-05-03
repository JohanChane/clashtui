use core::cell::{OnceCell, RefCell};
use std::{path::PathBuf, rc::Rc};

use crate::utils::{CfgError, Flag, Flags};
use ui::event;

use crate::msgpopup_methods;
use crate::tui::{
    tabs::{ClashSrvCtlTab, ProfileTab, TabEvent, Tabs},
    tools,
    utils::{HelpPopUp, InfoPopUp, Keys},
    widgets::MsgPopup,
    EventState, StatusBar, TabBar, Theme, Visibility,
};
use crate::utils::{ClashBackend, SharedBackend, SharedState, State};

pub struct App {
    tabbar: TabBar,
    tabs: Vec<Tabs>,
    pub should_quit: bool,
    help_popup: OnceCell<Box<HelpPopUp>>,
    info_popup: InfoPopUp,
    msgpopup: MsgPopup,

    util: SharedBackend,
    statusbar: StatusBar,
}

impl App {
    pub fn new(util: ClashBackend) -> Self {
        let util = SharedBackend::new(util);

        let state =
            SharedState::new(RefCell::new(State::new(Rc::clone(&util))));
        let _ = Theme::load(None).map_err(|e| log::error!("Loading Theme:{e}"));

        let tabs: Vec<Tabs> = vec![
            Tabs::Profile(ProfileTab::new(
                util.clone(),
                state.clone(),
            )),
            Tabs::ClashSrvCtl(ClashSrvCtlTab::new(
                util.clone(),
                state.clone(),
            )),
        ]; // Init the tabs
        let tabbar = TabBar::new(tabs.iter().map(|v| v.to_string()).collect());
        let statusbar = StatusBar::new(Rc::clone(&state));
        let info_popup = InfoPopUp::with_items(&util.clash_version());

        let app = Self {
            tabbar,
            should_quit: false,
            help_popup: Default::default(),
            info_popup,
            msgpopup: Default::default(),
            statusbar,
            util,
            tabs,
        };

        app
    }

    pub fn run(&mut self, err_track: Vec<CfgError>, flags: Flags<Flag>) -> std::io::Result<()> {
        const TICK_RATE: u64 = 250;
        use core::time::Duration;
        if flags.contains(Flag::FirstInit) {
            self.popup_txt_msg("Welcome to ClashTui(forked)!".to_string());
            self.popup_txt_msg(
                "Please go to Config Tab to set configs so that program can work properly"
                    .to_string(),
            );
        };
        if flags.contains(Flag::ErrorDuringInit) {
            self.popup_txt_msg(
                "Some Error happened during app init, Check the log for detail".to_string(),
            );
        }
        err_track
            .into_iter()
            .for_each(|e| self.popup_txt_msg(e.reason));
        log::info!("App init finished");

        use ratatui::{backend::CrosstermBackend, Terminal};
        let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
        let tick_rate = Duration::from_millis(TICK_RATE);
        while !self.should_quit {
            terminal.draw(|f| self.draw(f))?;

            self.late_event();

            if event::poll(tick_rate)? {
                if let Err(e) = self.event(&event::read()?) {
                    self.popup_txt_msg(e.to_string())
                };
            }
        }
        log::info!("App Exit");
        Ok(())
    }

    fn popup_event(&mut self, ev: &event::Event) -> Result<EventState, ui::Infailable> {
        // ## Self Popups
        let mut event_state = self
            .help_popup
            .get_mut()
            .and_then(|v| v.event(ev).ok())
            .unwrap_or(EventState::NotConsumed);
        if event_state.is_notconsumed() {
            event_state = self.info_popup.event(ev)?;
        }
        // ## Tab Popups
        let mut iter = self.tabs.iter_mut().map(|v| match v {
            Tabs::Profile(tab) => tab.popup_event(ev),
            Tabs::ClashSrvCtl(tab) => tab.popup_event(ev),
        });
        while event_state.is_notconsumed() {
            match iter.next() {
                Some(v) => event_state = v?,
                None => break,
            }
        }

        Ok(event_state)
    }

    pub fn event(&mut self, ev: &event::Event) -> Result<EventState, std::io::Error> {
        let mut event_state = self.msgpopup.event(ev)?;
        if event_state.is_notconsumed() {
            event_state = self.popup_event(ev)?;
        }
        if event_state.is_consumed() {
            return Ok(event_state);
        }

        if let event::Event::Key(key) = ev {
            if key.kind != event::KeyEventKind::Press {
                return Ok(EventState::NotConsumed);
            }
            event_state = match key.code.into() {
                Keys::AppQuit => {
                    self.should_quit = true;
                    EventState::WorkDone
                }
                Keys::AppHelp => {
                    self.help_popup.get_or_init(|| Box::new(HelpPopUp::new()));
                    self.help_popup.get_mut().unwrap().show();
                    EventState::WorkDone
                }
                Keys::AppInfo => {
                    self.info_popup.show();
                    EventState::WorkDone
                }
                Keys::ClashConfig => {
                    let _ = self
                        .util
                        .open_dir(self.util.home_dir.as_path())
                        .map_err(|e| log::error!("ODIR: {}", e));
                    EventState::WorkDone
                }
                Keys::AppConfig => {
                    let _ = self
                        .util
                        .open_dir(&PathBuf::from(&self.util.cfg.clash_cfg_dir))
                        .map_err(|e| log::error!("ODIR: {}", e));
                    EventState::WorkDone
                }
                Keys::LogCat => {
                    let log = self.util.fetch_recent_logs(20);
                    self.popup_list_msg(log);
                    EventState::WorkDone
                }
                Keys::SoftRestart => {
                    match self.util.restart_clash() {
                        Ok(output) => {
                            self.popup_list_msg(output.lines().map(|line| line.trim().to_string()));
                        }
                        Err(err) => {
                            self.popup_txt_msg(err.to_string());
                        }
                    }
                    EventState::WorkDone
                }
                _ => EventState::NotConsumed,
            };

            if event_state == EventState::NotConsumed {
                event_state = self
                    .tabbar
                    .event(ev)
                    .map_err(|()| std::io::Error::new(std::io::ErrorKind::Other, "Undefined"))?;
                let mut iter = self.tabs.iter_mut().map(|v| match v {
                    Tabs::Profile(tab) => tab.event(ev),
                    Tabs::ClashSrvCtl(tab) => Ok(tab.event(ev)?),
                });
                while event_state.is_notconsumed() {
                    match iter.next() {
                        Some(v) => event_state = v?,
                        None => break,
                    }
                }
            }
        }

        Ok(event_state)
    }
    pub fn late_event(&mut self) {
        self.tabs.iter_mut().for_each(|v| match v {
            Tabs::Profile(tab) => tab.late_event(),
            Tabs::ClashSrvCtl(tab) => tab.late_event(),
        })
    }

    pub fn draw(&mut self, f: &mut ratatui::prelude::Frame) {
        use ratatui::prelude as Ra;
        let chunks = Ra::Layout::default()
            .constraints(
                [
                    Ra::Constraint::Length(3),
                    Ra::Constraint::Min(0),
                    Ra::Constraint::Length(3),
                ]
                .as_ref(),
            )
            .split(f.size());

        self.update_tabbar();
        self.tabbar.draw(f, chunks[0]);

        let tab_chunk = chunks[1];
        self.tabs.iter_mut().for_each(|v| match v {
            Tabs::Profile(tab) => tab.draw(f, tab_chunk),
            Tabs::ClashSrvCtl(tab) => tab.draw(f, tab_chunk),
        });

        self.statusbar.draw(f, chunks[2]);

        let help_area = tools::centered_percent_rect(60, 60, f.size());
        if let Some(v) = self.help_popup.get_mut() {
            v.draw(f, help_area)
        }
        self.info_popup.draw(f, help_area);
        self.msgpopup.draw(f, help_area);
    }

    fn update_tabbar(&mut self) {
        let tabname = self
            .tabbar
            .selected()
            .expect("UB: selected tab out of bound");
        self.tabs
            .iter_mut()
            .map(|v| (v == tabname, v))
            .for_each(|(b, v)| match v {
                Tabs::Profile(tab) => tab.set_visible(b),
                Tabs::ClashSrvCtl(tab) => tab.set_visible(b),
            });
    }

    pub fn save(&self, config_path: &str) -> std::io::Result<()> {
        self.util
            .cfg
            .to_file(config_path)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }
}

msgpopup_methods!(App);
