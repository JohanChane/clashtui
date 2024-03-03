use core::cell::RefCell;
use crossterm::event::{Event, KeyEventKind};
use ratatui::prelude as Ra;
use std::{collections::HashMap, path::PathBuf, rc::Rc};

use crate::msgpopup_methods;
use crate::tui::{
    tabs::{ClashSrvCtlTab, ProfileTab, Tab, Tabs},
    tools,
    utils::{HelpPopUp, Keys},
    widgets::MsgPopup,
    EventState, StatusBar, TabBar, Theme, Visibility,
};
use crate::utils::{
    CfgError, ClashTuiUtil, Flag, Flags, SharedClashTuiState, SharedClashTuiUtil, State,
};

pub struct App {
    tabbar: TabBar,
    tabs: HashMap<Tab, Tabs>,
    pub should_quit: bool,
    help_popup: Box<HelpPopUp>,
    msgpopup: MsgPopup,

    clashtui_util: SharedClashTuiUtil,
    clashtui_state: SharedClashTuiState,
    statusbar: StatusBar,
}

impl App {
    pub fn new(flags: &Flags, clashtui_config_dir: &PathBuf) -> (Option<Self>, Vec<CfgError>) {
        #[cfg(debug_assertions)]
        let _ = std::fs::remove_file(clashtui_config_dir.join("clashtui.log")); // auto rm old log for debug
        setup_logging(clashtui_config_dir.join("clashtui.log").to_str().unwrap());

        let (util, mut err_track) = ClashTuiUtil::new(
            clashtui_config_dir,
            &clashtui_config_dir.join("profiles"),
            !flags.contains(Flag::FirstInit),
        );
        if flags.contains(Flag::UpdateOnly) {
            let log_path = &clashtui_config_dir.join("CronUpdate.log");
            let _ = std::fs::remove_file(log_path); // clear old logs
            log::info!("Cron Mode!");
            println!("Log saved to CronUpdate.log");
            let profile_list: Vec<_> = util
                .get_profile_names()
                .unwrap()
                .iter()
                .map(|v| util.update_local_profile(v, false))
                .collect();
            let mut x = match std::fs::File::create(log_path) {
                Err(e) => {
                    log::error!("Err while CronUpdate: {}", e);
                    err_track.push(CfgError::new(
                        crate::utils::ErrKind::CronUpdateProfile,
                        e.to_string(),
                    ));
                    return (None, err_track);
                }
                Ok(v) => v,
            };
            let _ = std::io::Write::write_all(
                &mut x,
                format!(
                    "{:?}",
                    profile_list
                        .into_iter()
                        .flat_map(|v| match v {
                            Ok(v) => crate::utils::concat_update_profile_result(v),
                            Err(e) => vec![e.to_string()],
                        })
                        .collect::<Vec<String>>()
                )
                .as_bytes(),
            )
            .map_err(|e| log::error!("Err while CronUpdate: {}", e));
            return (None, err_track);
        } // Finish cron
        let clashtui_util = SharedClashTuiUtil::new(util);

        let clashtui_state =
            SharedClashTuiState::new(RefCell::new(State::new(Rc::clone(&clashtui_util))));
        let _ = Theme::load(None).map_err(|e| log::error!("Loading Theme:{}", e));
        // May not often used, place in heap
        let help_popup = Box::new(HelpPopUp::new());

        let tabs_ = [Tab::Profile, Tab::ClashSrvCtl];
        let tabbar = TabBar::new(tabs_.iter().map(|v| v.to_string()).collect());
        let tabs: HashMap<Tab, Tabs> = HashMap::from_iter(tabs_.into_iter().zip([
            Tabs::Profile(RefCell::new(ProfileTab::new(
                clashtui_util.clone(),
                clashtui_state.clone(),
            ))),
            Tabs::ClashSrvCtl(RefCell::new(ClashSrvCtlTab::new(
                clashtui_util.clone(),
                clashtui_state.clone(),
            ))),
        ])); // Init the tabs
        let statusbar = StatusBar::new(Rc::clone(&clashtui_state));

        let app = Self {
            tabbar,
            should_quit: false,
            help_popup,
            msgpopup: MsgPopup::new(),
            statusbar,
            clashtui_util,
            clashtui_state,
            tabs,
        };

        (Some(app), err_track)
    }

    fn popup_event(&mut self, ev: &Event) -> Result<EventState, ui::Infailable> {
        // ## Self Popups
        let mut event_state = self.help_popup.event(ev)?;

        // ## Tab Popups
        let mut iter = self.tabs.values().map(|v| match v {
            Tabs::Profile(v) => v.borrow_mut().popup_event(ev),
            Tabs::ClashSrvCtl(v) => v.borrow_mut().popup_event(ev),
        });
        while event_state.is_notconsumed() {
            match iter.next() {
                Some(v) => event_state = v?,
                None => break,
            }
        }

        Ok(event_state)
    }

    pub fn event(&mut self, ev: &Event) -> Result<EventState, std::io::Error> {
        let mut event_state = self.msgpopup.event(ev)?;
        if event_state.is_notconsumed() {
            event_state = self.popup_event(ev)?;
        }
        if event_state.is_consumed() {
            return Ok(event_state);
        }

        if let Event::Key(key) = ev {
            if key.kind != KeyEventKind::Press {
                return Ok(EventState::NotConsumed);
            }
            event_state = match key.code.into() {
                Keys::AppQuit => {
                    self.should_quit = true;
                    EventState::WorkDone
                }
                Keys::AppHelp => {
                    self.help_popup.show();
                    EventState::WorkDone
                }
                Keys::ClashConfig => {
                    let _ = self
                        .clashtui_util
                        .open_dir(self.clashtui_util.clashtui_dir.as_path())
                        .map_err(|e| log::error!("ODIR: {}", e));
                    EventState::WorkDone
                }
                Keys::AppConfig => {
                    let _ = self
                        .clashtui_util
                        .open_dir(&PathBuf::from(&self.clashtui_util.tui_cfg.clash_cfg_dir))
                        .map_err(|e| log::error!("ODIR: {}", e));
                    EventState::WorkDone
                }
                Keys::LogCat => {
                    let log = self.clashtui_util.fetch_recent_logs(20);
                    self.popup_list_msg(log);
                    EventState::WorkDone
                }
                Keys::ClashsrvctlRestart => {
                    match self.clashtui_util.restart_clash() {
                        Ok(output) => {
                            let list_msg: Vec<String> =
                                output.lines().map(|line| line.trim().to_string()).collect();
                            self.popup_list_msg(list_msg);
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
                let mut iter = self.tabs.values().map(|v| match v {
                    Tabs::Profile(v) => v.borrow_mut().event(ev),
                    Tabs::ClashSrvCtl(v) => Ok(v.borrow_mut().event(ev)?),
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

    // For refreshing the interface before performing lengthy operation.
    pub fn handle_last_ev(&mut self, last_ev: &EventState) -> EventState {
        let ev_state = match last_ev {
            EventState::NotConsumed | EventState::WorkDone => EventState::NotConsumed,
            EventState::Yes | EventState::Cancel => unreachable!(),
            EventState::ProfileUpdate | EventState::ProfileUpdateAll => {
                if let Tabs::Profile(profile_tab) = self.tabs.get(&Tab::Profile).unwrap() {
                    profile_tab.borrow_mut().hide_msgpopup();
                    if last_ev == &EventState::ProfileUpdate {
                        profile_tab.borrow_mut().handle_update_profile_ev(false);
                    } else if last_ev == &EventState::ProfileUpdateAll {
                        profile_tab.borrow_mut().handle_update_profile_ev(true);
                    }
                };
                EventState::WorkDone
            }
            EventState::ProfileSelect => {
                if let Tabs::Profile(profile_tab) = self.tabs.get(&Tab::Profile).unwrap() {
                    profile_tab.borrow_mut().hide_msgpopup();
                    if let Some(v) = profile_tab.borrow_mut().handle_select_profile_ev() {
                        self.clashtui_state.borrow_mut().set_profile(v)
                    }
                };
                EventState::WorkDone
            }
            EventState::ProfileDelete => {
                if let Tabs::Profile(profile_tab) = self.tabs.get(&Tab::Profile).unwrap() {
                    profile_tab.borrow_mut().hide_msgpopup();
                    profile_tab.borrow_mut().handle_delete_profile_ev();
                };
                EventState::WorkDone
            }
            #[cfg(target_os = "windows")]
            EventState::SwitchSysProxy => {
                let cur = self
                    .clashtui_state
                    .borrow()
                    .get_sysproxy()
                    .map_or(true, |b| !b);
                self.clashtui_state.borrow_mut().set_sysproxy(cur);
                EventState::WorkDone
            }
        };

        ev_state
    }

    pub fn draw(&mut self, f: &mut Ra::Frame) {
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
        self.tabs.values().for_each(|v| match v {
            Tabs::Profile(v) => v.borrow_mut().draw(f, tab_chunk),
            Tabs::ClashSrvCtl(v) => v.borrow_mut().draw(f, tab_chunk),
        });

        self.statusbar.draw(f, chunks[2]);

        let help_area = tools::centered_percent_rect(60, 60, f.size());
        self.help_popup.draw(f, help_area);
        self.msgpopup.draw(f, help_area);
    }

    pub fn on_tick(&mut self) {}

    fn update_tabbar(&self) {
        let tabname = self
            .tabbar
            .selected()
            .expect("UB: selected tab out of bound");
        self.tabs
            .iter()
            .map(|(n, v)| (n == tabname, v))
            .for_each(|(b, v)| match v {
                Tabs::Profile(k) => k.borrow_mut().set_visible(b),
                Tabs::ClashSrvCtl(k) => k.borrow_mut().set_visible(b),
            });
    }

    pub fn save(&self, config_path: &str) -> Result<(), CfgError> {
        self.clashtui_util.tui_cfg.to_file(config_path)
    }
}

fn setup_logging(log_path: &str) {
    use log4rs::append::file::FileAppender;
    use log4rs::config::{Appender, Config, Root};
    use log4rs::encode::pattern::PatternEncoder;
    // No need to change. This is set to auto switch to Info level when build release
    #[allow(unused_variables)]
    let log_level = log::LevelFilter::Info;
    #[cfg(debug_assertions)]
    let log_level = log::LevelFilter::Debug;
    let file_appender = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("[{l}] {t} - {m}{n}")))
        .build(log_path)
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("file", Box::new(file_appender)))
        .build(Root::builder().appender("file").build(log_level))
        .unwrap();

    log4rs::init_config(config).unwrap();
    log::info!("Start Log, level: {}", log_level);
}

msgpopup_methods!(App);
