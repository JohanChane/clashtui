use anyhow::Result;
use crossterm::event::{Event, KeyEventKind};
use log;
use ratatui::prelude as Ra;
use std::{cell::RefCell, collections::HashMap, env, path::PathBuf, rc::Rc};

use crate::msgpopup_methods;
use crate::ui::popups::{HelpPopUp, MsgPopup};
use crate::ui::tabs::{ClashSrvCtlTab, CommonTab, ConfigTab, ProfileTab, Tab, Tabs};
use crate::ui::utils::{symbols, tools, Keys, Theme, Visibility};
use crate::ui::{ClashTuiStatusBar, ClashTuiTabBar, EventState};
use crate::utils::{ClashTuiUtil, Flags, SharedClashTuiState, SharedClashTuiUtil, State, Utils};

pub struct App {
    title: String,
    tabbar: ClashTuiTabBar,
    tabs: HashMap<Tab, Tabs>,
    pub should_quit: bool,
    help_popup: HelpPopUp,
    msgpopup: MsgPopup,

    clashtui_util: SharedClashTuiUtil,
    clashtui_state: SharedClashTuiState,
    statusbar: ClashTuiStatusBar,
    pub flags: HashMap<Flags, bool>,
}

impl App {
    pub fn new(mut flags: HashMap<Flags, bool>) -> Option<Self> {
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
            if let Err(err) = crate::utils::init_config(
                &clashtui_config_dir,
                symbols::DEFAULT_BASIC_CLASH_CFG_CONTENT,
            ) {
                flags.insert(Flags::ErrorDuringInit, true);
                log::error!("{}", err);
            }
        } else {
            flags.insert(Flags::FirstInit, false);
        }

        #[cfg(debug_assertions)]
        let _ = std::fs::remove_file(&clashtui_config_dir.join("clashtui.log")); // auto rm old log for debug
        Self::setup_logging(&clashtui_config_dir.join("clashtui.log").to_str().unwrap());

        let clashtui_util = Rc::new(ClashTuiUtil::new(
            &clashtui_config_dir,
            &clashtui_config_dir.join("profiles"),
            *flags.get(&Flags::FirstInit).unwrap(),
        ));
        if *flags.get(&Flags::UpdateOnly).unwrap() {
            let log_path = &clashtui_config_dir.join("CronUpdate.log");
            let _ = std::fs::remove_file(log_path); // clear old logs
            log::info!("Cron Mode!");
            println!("Log saved to CronUpdate.log");
            let profile_list: Vec<_> = clashtui_util
                .get_profile_names()
                .unwrap()
                .iter()
                .map(|v| clashtui_util.update_local_profile(v, false))
                .collect();
            let mut x = std::fs::File::create(log_path)
                .map_err(|e| log::error!("Err while CronUpdate: {}", e))
                .unwrap();
            let _ = std::io::Write::write_all(
                &mut x,
                format!(
                    "{:?}",
                    profile_list.into_iter().map(|v| match v {
                        Ok(v) => Utils::concat_update_profile_result(v),
                        Err(e) => [e.to_string()].to_vec(),
                    }).flatten().collect::<Vec<String>>()
                )
                .as_bytes(),
            )
            .map_err(|e| log::error!("Err while CronUpdate: {}", e));
            return None;
        } // Finish cron

        let theme = Rc::new(Theme::default());

        let help_popup = HelpPopUp::new("Help".to_string(), Rc::clone(&theme));

        let clashtui_state =
            SharedClashTuiState::new(RefCell::new(State::new(clashtui_util.clone())));

        let statusbar = ClashTuiStatusBar::new(Rc::clone(&clashtui_state), Rc::clone(&theme));

        let mut tabs: HashMap<Tab, Tabs> = HashMap::with_capacity(3);
        // Init the tabs
        {
            tabs.insert(
                Tab::ProfileTab,
                Tabs::ProfileTab(RefCell::new(ProfileTab::new(
                    symbols::PROFILE.to_string(),
                    clashtui_util.clone(),
                    clashtui_state.clone(),
                    theme.clone(),
                ))),
            );
            tabs.insert(
                Tab::ClashSrvCtlTab,
                Tabs::ClashSrvCtlTab(RefCell::new(ClashSrvCtlTab::new(
                    symbols::CLASHSRVCTL.to_string(),
                    clashtui_util.clone(),
                    clashtui_state.clone(),
                    theme.clone(),
                ))),
            );
            tabs.insert(
                Tab::ConfigTab,
                Tabs::ConfigTab(RefCell::new(ConfigTab::new(
                    symbols::CONFIG.to_string(),
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
                    symbols::PROFILE.to_string(),
                    symbols::CLASHSRVCTL.to_string(),
                    symbols::CONFIG.to_string(),
                ],
                Rc::clone(&theme),
            ),
            should_quit: false,
            help_popup,
            msgpopup: MsgPopup::new(),
            statusbar,
            clashtui_util,
            clashtui_state,
            tabs,
            flags,
        };

        let help_text: Vec<String> = symbols::HELP // TODO
            .lines()
            .map(|line| line.trim().to_string())
            .collect();
        app.help_popup.set_items(help_text);
        if app.flags.get(&Flags::ErrorDuringInit).is_none() {
            app.flags.insert(Flags::ErrorDuringInit, false);
        }

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
            } else if Keys::ClashConfig.is(key) {
                let _ = self
                    .clashtui_util
                    .open_dir(self.clashtui_util.clashtui_dir.as_path())
                    .map_err(|e| log::error!("ODIR: {}", e));
                EventState::WorkDone
            } else if Keys::AppConfig.is(key) {
                let _ = self
                    .clashtui_util
                    .open_dir(&PathBuf::from(
                        self.clashtui_util
                            .get_cfg(crate::utils::CfgOp::ClashConfigDir),
                    ))
                    .map_err(|e| log::error!("ODIR: {}", e));
                EventState::WorkDone
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
                if let Tabs::ProfileTab(profile_tab) = self.tabs.get(&Tab::ProfileTab).unwrap() {
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
                if let Tabs::ProfileTab(profile_tab) = self.tabs.get(&Tab::ProfileTab).unwrap() {
                    profile_tab.borrow_mut().hide_msgpopup();
                    match profile_tab.borrow_mut().handle_select_profile_ev() {
                        Some(v) => self.clashtui_state.borrow_mut().set_profile(v),
                        None => (),
                    };
                };
                EventState::WorkDone
            }
            EventState::ProfileDelete => {
                if let Tabs::ProfileTab(profile_tab) = self.tabs.get(&Tab::ProfileTab).unwrap() {
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

    pub fn save_config(&self) {
        self.clashtui_util.save_cfg()
    }

    pub fn get_err_track(&self) -> Vec<crate::utils::ClashTuiConfigLoadError> {
        self.clashtui_util.get_err_track()
    }
}

msgpopup_methods!(App);
