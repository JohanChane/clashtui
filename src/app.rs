use anyhow::Result;
use crossterm::event::{Event, KeyEventKind};
use log;
use ratatui::prelude as Ra;
use std::{cell::RefCell, collections::HashMap, env, path::PathBuf, rc::Rc};

use crate::msgpopup_methods;
use crate::ui::popups::{HelpPopUp, MsgPopup};
use crate::ui::tabs::{ClashSrvCtlTab, CommonTab, ConfigTab, ProfileTab, Tabs};
use crate::ui::utils::{tools, Keys, SharedSymbols, Symbols, Theme, Visibility};
use crate::ui::{ClashTuiStatusBar, ClashTuiTabBar, EventState};
use crate::utils::{ClashTuiUtil, Flags, SharedClashTuiState, SharedClashTuiUtil, State};

pub struct App {
    title: String,
    tabbar: ClashTuiTabBar,
    tabs: HashMap<String, Tabs>,
    pub should_quit: bool,
    help_popup: HelpPopUp,
    msgpopup: MsgPopup,

    symbols: SharedSymbols,
    clashtui_util: SharedClashTuiUtil,
    clashtui_state: SharedClashTuiState,
    statusbar: ClashTuiStatusBar,
    pub flags: HashMap<Flags, bool>,
}

impl App {
    pub fn new(mut flags: HashMap<Flags, bool>) -> Option<Self> {
        let names = Rc::new(Symbols::default());

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
                flags.insert(Flags::ErrorDuringInit, true);
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
        if *flags.get(&Flags::UpdateOnly).unwrap() {
            log::info!("Cron Mode!");
            let profile_list:Vec<_> = clashtui_util
                .get_profile_names()
                .unwrap()
                .iter()
                .map(|v| clashtui_util.update_local_profile(v, false)).collect();
            let mut x = std::fs::File::create(&clashtui_config_dir.join("CronUpdate.log")).map_err(|e|log::error!("Err while CronUpdate: {}", e)).unwrap();
            let _ = std::io::Write::write_all(&mut x, format!("{:?}",profile_list).as_bytes()).map_err(|e|log::error!("Err while CronUpdate: {}", e));
            drop(x);
            drop(profile_list);
            return None;
        }

        let theme = Rc::new(Theme::default());

        let help_popup = HelpPopUp::new("Help".to_string(), Rc::clone(&theme));

        let clashtui_state =
            SharedClashTuiState::new(RefCell::new(State::new(clashtui_util.clone())));

        let statusbar = ClashTuiStatusBar::new(Rc::clone(&clashtui_state), Rc::clone(&theme));

        let mut tabs: HashMap<String, Tabs> = HashMap::with_capacity(3);
        {
            // Init the tabs
            tabs.insert(
                names.profile.clone(),
                Tabs::ProfileTab(RefCell::new(ProfileTab::new(
                    names.profile.clone(),
                    clashtui_util.clone(),
                    clashtui_state.clone(),
                    theme.clone(),
                ))),
            );
            tabs.insert(
                names.clashsrvctl.clone(),
                Tabs::ClashSrvCtlTab(RefCell::new(ClashSrvCtlTab::new(
                    names.clashsrvctl.clone(),
                    Rc::clone(&clashtui_util),
                    Rc::clone(&theme),
                ))),
            );
            tabs.insert(
                names.config.clone(),
                Tabs::ConfigTab(RefCell::new(ConfigTab::new(
                    names.config.clone(),
                    clashtui_util.clone(),
                    theme.clone(),
                ))),
            );
        }

        let mut app = Self {
            title: "ClashTui".to_string(),
            tabbar: ClashTuiTabBar::new(
                "".to_string(),
                vec![
                    names.profile.clone(),
                    names.clashsrvctl.clone(),
                    names.config.to_string(),
                ],
                Rc::clone(&theme),
            ),
            should_quit: false,
            symbols: names,
            help_popup,
            msgpopup: MsgPopup::new(),
            statusbar,
            clashtui_util,
            clashtui_state,
            tabs,
            flags,
        };

        let help_text: Vec<String> = app
            .symbols
            .help
            .lines()
            .map(|line| line.trim().to_string())
            .collect();
        app.help_popup.set_items(help_text);
        app.flags.insert(Flags::ErrorDuringInit, false);

        Some(app)
    }

    fn popup_event(&mut self, ev: &Event) -> Result<EventState> {
        // ## Self Popups
        let mut event_state = self.help_popup.event(ev).unwrap();

        // ## Tab Popups
        let mut iter = self.tabs.values().map(|v| match v {
            Tabs::ProfileTab(v) => v.borrow_mut().popup_event(ev).unwrap(),
            Tabs::ClashSrvCtlTab(v) => v.borrow_mut().popup_event(ev).unwrap(),
            Tabs::ConfigTab(v) => v.borrow_mut().popup_event(ev).unwrap(),
        });
        let mut tmp;
        while event_state.is_notconsumed() {
            tmp = iter.next();
            match tmp {
                Some(v) => event_state = v,
                None => break,
            }
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

            event_state = if Keys::AppQuit.is(key) {
                self.should_quit = true;
                EventState::WorkDone
            } else if Keys::AppHelp.is(key) {
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
            } else if Keys::LogCat.is(key) {
                let log = self.clashtui_util.fetch_recent_logs(20);
                self.popup_list_msg(log);
                EventState::WorkDone
            } else if Keys::ClashsrvctlRestart.is(key) {
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
            } else {
                EventState::NotConsumed
            };

            if event_state == EventState::NotConsumed {
                event_state = self.tabbar.event(ev)?;
                let mut iter = self.tabs.values().map(|v| match v {
                    Tabs::ProfileTab(v) => v.borrow_mut().event(ev).unwrap(),
                    Tabs::ClashSrvCtlTab(v) => v.borrow_mut().event(ev).unwrap(),
                    Tabs::ConfigTab(v) => v.borrow_mut().event(ev).unwrap(),
                });
                let mut tmp;
                while event_state.is_notconsumed() {
                    tmp = iter.next();
                    match tmp {
                        Some(v) => event_state = v,
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
            EventState::ProfileUpdate | EventState::ProfileUpdateAll => {
                if let Tabs::ProfileTab(profile_tab) = self.tabs.get(&self.symbols.profile).unwrap()
                {
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
                if let Tabs::ProfileTab(profile_tab) = self.tabs.get(&self.symbols.profile).unwrap()
                {
                    profile_tab.borrow_mut().hide_msgpopup();
                    match profile_tab.borrow_mut().handle_select_profile_ev() {
                        Some(v) => self.clashtui_state.borrow_mut().set_profile(v),
                        None => (),
                    };
                };
                EventState::WorkDone
            }
            EventState::ProfileDelete => {
                if let Tabs::ProfileTab(profile_tab) = self.tabs.get(&self.symbols.profile).unwrap()
                {
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

        let tab_chunk = chunks[1];
        self.tabs
            .values()
            .map(|v| match v {
                Tabs::ProfileTab(v) => v.borrow_mut().draw(f, tab_chunk),
                Tabs::ClashSrvCtlTab(v) => v.borrow_mut().draw(f, tab_chunk),
                Tabs::ConfigTab(v) => v.borrow_mut().draw(f, tab_chunk),
            })
            .count();

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
        let _ = self
            .tabs
            .iter()
            .map(|(n, v)| if n == tabname { (true, v) } else { (false, v) })
            .map(|(b, v)| match v {
                Tabs::ProfileTab(k) => k.borrow_mut().set_visible(b),
                Tabs::ClashSrvCtlTab(k) => k.borrow_mut().set_visible(b),
                Tabs::ConfigTab(k) => k.borrow_mut().set_visible(b),
            })
            .count();
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

    pub fn save_config(&self) {
        self.clashtui_util.save_config()
    }

    pub fn get_err_track(&self) -> Vec<crate::utils::ClashTuiConfigLoadError> {
        self.clashtui_util.get_err_track()
    }
}

msgpopup_methods!(App);
