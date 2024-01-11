use anyhow::Result;
use crossterm::event::{Event, KeyEventKind};
use log;
use ratatui::prelude as Ra;
use std::{cell::RefCell, collections::HashMap, env, path::PathBuf, rc::Rc};

use crate::msgpopup_methods;
use crate::ui::keys::{match_key, KeyList, SharedKeyList};
use crate::ui::popups::{ClashTuiListPopup, MsgPopup};
use crate::ui::tabs::{ClashSrvCtlTab, CommonTab, ConfigTab, ProfileTab, Tab, Tabs};
use crate::ui::utils::{helper, Theme};
use crate::ui::{ClashTuiStatusBar, ClashTuiTabBar, EventState, SharedSymbols, Symbols};
use crate::utils::{
    ClashTuiOp, ClashTuiUtil, Flags, SharedClashTuiState, SharedClashTuiUtil, State,
};

pub struct App {
    title: String,
    tabbar: ClashTuiTabBar,
    tabs: Vec<Tabs>,
    pub should_quit: bool,
    pub help_popup: ClashTuiListPopup,
    pub msgpopup: MsgPopup,

    pub key_list: SharedKeyList,
    pub symbols: SharedSymbols,
    pub clashtui_util: SharedClashTuiUtil,
    clashtui_state: SharedClashTuiState,
    pub statusbar: ClashTuiStatusBar,
    pub flags: HashMap<Flags, bool>,
}

impl App {
    pub fn new() -> Self {
        let mut flags: HashMap<Flags, bool> = HashMap::with_capacity(1);
        let key_list = Rc::new(KeyList::default());
        let names = Rc::new(Symbols::default());
        let theme = Rc::new(Theme::default());

        let exe_dir = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();

        let data_dir = exe_dir.join("data");
        let clashtui_config_dir = if data_dir.exists() && data_dir.is_dir() {
            // portable mode
            log::info!("Portable Mode!");
            data_dir
        } else {
            #[cfg(target_os = "linux")]
            let clashtui_config_dir_str = env::var("XDG_CONFIG_HOME")
                .or_else(|_| env::var("HOME").map(|home| format!("{}/.config/clashtui", home)))
                .unwrap();
            #[cfg(target_os = "windows")]
            let clashtui_config_dir_str = env::var("APPDATA")
                .map(|appdata| format!("{}/clashtui", appdata))
                .unwrap();
            PathBuf::from(&clashtui_config_dir_str)
        };

        if !clashtui_config_dir.join("config.yaml").exists() {
            flags.insert(Flags::FirstInit, true);
            if let Err(err) = crate::utils::init_config(&clashtui_config_dir, &names) {
                log::error!("{}", err);
            }
        } else {
            flags.insert(Flags::FirstInit, false);
        }

        #[cfg(debug_assertions)]
        let _ = std::fs::remove_file(&clashtui_config_dir.join("clashtui.log"));

        Self::setup_logging(&clashtui_config_dir.join("clashtui.log").to_str().unwrap());

        let clashtui_util = Rc::new(ClashTuiUtil::new(
            &clashtui_config_dir,
            &clashtui_config_dir.join("profiles"),
            *flags.get(&Flags::FirstInit).unwrap(),
        ));

        let help_popup = ClashTuiListPopup::new("Help".to_string(), Rc::clone(&theme));

        let clashtui_state =
            SharedClashTuiState::new(RefCell::new(State::new(clashtui_util.clone())));

        let statusbar = ClashTuiStatusBar::new(Rc::clone(&clashtui_state), Rc::clone(&theme));

        let mut tab: Vec<Tabs> = Vec::with_capacity(3);
        tab.push(Tabs::ProfileTab(RefCell::new(ProfileTab::new(
            key_list.clone(),
            names.clone(),
            clashtui_util.clone(),
            clashtui_state.clone(),
            theme.clone(),
        ))));
        tab.push(Tabs::ClashSrvCtlTab(RefCell::new(ClashSrvCtlTab::new(
            key_list.clone(),
            names.clone(),
            Rc::clone(&clashtui_util),
            Rc::clone(&theme),
        ))));
        tab.push(Tabs::ConfigTab(RefCell::new(ConfigTab::new(
            names.clone(),
            clashtui_util.clone(),
            theme.clone(),
        ))));

        let mut app = Self {
            title: "ClashTui".to_string(),
            tabbar: ClashTuiTabBar::new(
                "".to_string(),
                tab.iter()
                    .map(|x| match x {
                        Tabs::ProfileTab(v) => v.borrow().get_title().clone(),
                        Tabs::ClashSrvCtlTab(v) => v.borrow().get_title().clone(),
                        Tabs::ConfigTab(v) => v.borrow().get_title().clone(),
                    })
                    .collect(),
                Rc::clone(&theme),
            ),
            should_quit: false,
            key_list,
            symbols: names,
            help_popup,
            msgpopup: MsgPopup::new(),
            statusbar,
            clashtui_util,
            clashtui_state,
            tabs: tab,
            flags,
        };

        let help_text: Vec<String> = app
            .symbols
            .help
            .lines()
            .map(|line| line.trim().to_string())
            .collect();
        app.help_popup.set_items(help_text);

        app
    }

    pub fn popup_event(&mut self, ev: &Event) -> Result<EventState> {
        // ## Self Popups
        let mut event_state = self.help_popup.event(ev).unwrap();

        // ## Tab Popups
        if event_state.is_notconsumed() {
            event_state = match self.tabs.get(0).unwrap() {
                Tabs::ProfileTab(v) => v.borrow_mut().popup_event(ev).unwrap(),
                _ => EventState::UnexpectedERROR,
            };
        }
        if event_state.is_notconsumed() {
            event_state = match self.tabs.get(1).unwrap() {
                Tabs::ClashSrvCtlTab(v) => v.borrow_mut().popup_event(ev).unwrap(),
                _ => EventState::UnexpectedERROR,
            };
        }
        if event_state.is_notconsumed() {
            event_state = match self.tabs.get(2).unwrap() {
                Tabs::ConfigTab(v) => v.borrow_mut().popup_event(ev).unwrap(),
                Tabs::ClashSrvCtlTab(_) => EventState::UnexpectedERROR,
                Tabs::ProfileTab(_) => EventState::UnexpectedERROR,
            };
        }

        Ok(event_state)
    }

    pub fn event(&mut self, ev: &Event) -> Result<EventState> {
        let mut event_state = self.msgpopup.event(ev).unwrap();
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

            event_state = if match_key(key, &self.key_list.app_quit) {
                self.should_quit = true;
                EventState::WorkDone
            } else if match_key(key, &self.key_list.app_help) {
                self.help_popup.show();
                EventState::WorkDone
            //} else if match_key(key, &self.key_list.app_home_open) {
            //    self.clashtui_util
            //        .open_dir(self.clashtui_util.clashtui_dir.as_path())?;
            //    EventState::WorkDone
            //} else if match_key(key, &self.key_list.clash_cfg_dir_open) {
            //    self.clashtui_util
            //        .open_dir(self.clashtui_util.clashtui_config.clash_cfg_dir.as_path())?;
            //    EventState::WorkDone
            } else if match_key(key, &self.key_list.log_cat) {
                let log = self.clashtui_util.fetch_recent_logs(20);
                self.popup_list_msg(log);
                EventState::WorkDone
            } else if match_key(key, &self.key_list.clashsrvctl_start) {
                match self.clashtui_util.clash_srv_ctl(ClashTuiOp::StartClash) {
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
            } else if match_key(key, &self.key_list.clashsrvctl_restart) {
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
            } else if match_key(key, &self.key_list.clashsrvctl_stop) {
                match self.clashtui_util.clash_srv_ctl(ClashTuiOp::StopClash) {
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
            } else {
                EventState::NotConsumed
            };

            if event_state == EventState::NotConsumed {
                event_state = self.tabbar.event(ev)?;
                if event_state.is_notconsumed() {
                    if let Tabs::ProfileTab(tab) = self.tab(Tab::ProfileTab) {
                        event_state = tab.borrow_mut().event(ev).unwrap();
                    }
                }
                if event_state.is_notconsumed() {
                    if let Tabs::ClashSrvCtlTab(tab) = self.tab(Tab::ClashSrvCtlTab) {
                        event_state = tab.borrow_mut().event(ev).unwrap();
                    }
                }
                if event_state.is_notconsumed() {
                    if let Tabs::ConfigTab(tab) = self.tab(Tab::ConfigTab) {
                        event_state = tab.borrow_mut().event(ev).unwrap();
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
            EventState::ProfileUpdate | EventState::ProfileUpdateAll => {
                if let Tabs::ProfileTab(profile_tab) = self.tab(Tab::ProfileTab) {
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
                if let Tabs::ProfileTab(profile_tab) = self.tab(Tab::ProfileTab) {
                    profile_tab.borrow_mut().hide_msgpopup();
                    match profile_tab.borrow_mut().handle_select_profile_ev() {
                        Some(v) => self.clashtui_state.borrow_mut().set_profile(v),
                        None => (),
                    };
                };
                EventState::WorkDone
            }
            EventState::ProfileDelete => {
                if let Tabs::ProfileTab(profile_tab) = self.tab(Tab::ProfileTab) {
                    profile_tab.borrow_mut().hide_msgpopup();
                    profile_tab.borrow_mut().handle_delete_profile_ev();
                };
                EventState::WorkDone
            }
            #[cfg(target_os = "windows")]
            EventState::EnableSysProxy => {
                self.clashsrvctl_tab.hide_msgpopup();
                self.clashtui_util.enable_system_proxy();
                self.clashtui_state.borrow_mut().set_sysproxy(true);
                EventState::WorkDone
            }
            #[cfg(target_os = "windows")]
            EventState::DisableSysProxy => {
                self.clashsrvctl_tab.hide_msgpopup();
                ClashTuiUtil::disable_system_proxy();
                self.clashtui_state.borrow_mut().set_sysproxy(false);
                EventState::WorkDone
            }
            _ => EventState::NotConsumed,
        };

        ev_state
    }

    pub fn draw<B: Ra::Backend>(&mut self, f: &mut Ra::Frame<B>) {
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

        let tabcontent_chunk = chunks[1];
        self.tabs
            .iter()
            .map(|v| match v {
                Tabs::ProfileTab(k) => k.borrow_mut().draw(f, tabcontent_chunk),
                Tabs::ClashSrvCtlTab(k) => k.borrow_mut().draw(f, tabcontent_chunk),
                Tabs::ConfigTab(k) => k.borrow_mut().draw(f, tabcontent_chunk),
            })
            .count();

        self.statusbar.draw(f, chunks[2]);

        let help_area = helper::centered_percent_rect(60, 60, f.size());
        self.help_popup.draw(f, help_area);
        self.msgpopup.draw(f, help_area);
    }

    pub fn on_tick(&mut self) {}

    pub fn update_tabbar(&mut self) {
        let tabname = self.tabbar.selected();
        let _ = self
            .tabs
            .iter()
            .map(|v| match v {
                Tabs::ProfileTab(k) => {
                    let mut l = k.borrow_mut();
                    if tabname == Some(l.get_title()) {
                        l.show()
                    } else {
                        l.hide()
                    }
                }
                Tabs::ClashSrvCtlTab(k) => {
                    let mut l = k.borrow_mut();
                    if tabname == Some(l.get_title()) {
                        l.show()
                    } else {
                        l.hide()
                    }
                }
                Tabs::ConfigTab(k) => {
                    let mut l = k.borrow_mut();
                    if tabname == Some(l.get_title()) {
                        l.show()
                    } else {
                        l.hide()
                    }
                }
            })
            .count();
    }

    fn tab(&self, name: Tab) -> &Tabs {
        let idx: usize;
        if name == Tab::ProfileTab {
            idx = 0;
        } else if name == Tab::ClashSrvCtlTab {
            idx = 1;
        } else if name == Tab::ConfigTab {
            idx = 2;
        } else {
            todo!();
        }
        match self.tabs.get(idx) {
            Some(v) => v,
            None => todo!(),
        }
    }

    fn setup_logging(log_path: &str) {
        use log4rs::append::file::FileAppender;
        use log4rs::config::{Appender, Config, Root};
        use log4rs::encode::pattern::PatternEncoder;
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
}

msgpopup_methods!(App);
