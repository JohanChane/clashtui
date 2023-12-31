use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use log;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use ratatui::style::{Color, Modifier, Style};
use ratatui::{prelude::*, widgets::*};
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::ops::{Deref, DerefMut};
use std::process::{Command, Output};
use std::rc::Rc;
use std::{
    fs::{self, read_dir, File},
    path::{Path, PathBuf},
};

use crate::clashtui_state::{ClashTuiState, SharedClashTuiState};
use crate::keys::{match_key, KeyList, SharedKeyList};
use crate::msgpopup_methods;
use crate::ui::clashsrvctl_tab::{self, ClashSrvCtlTab};
use crate::ui::profile_tab::ProfileTab;
use crate::ui::statusbar::ClashTuiStatusBar;
use crate::ui::ClashTuiOp;
use crate::ui::{
    widgets::{helper, ClashTuiListPopup, ClashTuiTabBar, SharedTheme, Theme},
    EventState, MsgPopup,
};
use crate::ui::{SharedSymbols, Symbols};
use crate::utils::{ClashTuiUtil, SharedClashTuiUtil};

pub struct App {
    pub title: String,
    pub tabbar: ClashTuiTabBar,
    pub profile_tab: ProfileTab,
    pub clashsrvctl_tab: ClashSrvCtlTab,
    pub should_quit: bool,
    pub help_popup: ClashTuiListPopup,
    pub msgpopup: MsgPopup,

    pub key_list: SharedKeyList,
    pub symbols: SharedSymbols,
    pub clashtui_util: SharedClashTuiUtil,
    pub clashtui_state: SharedClashTuiState,
    pub statusbar: ClashTuiStatusBar,
}

impl App {
    pub fn new() -> Self {
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

        if !clashtui_config_dir.exists(){ //.join("basic_clash_config.yaml").exists() { // weird, shouldn`t we check the dir rather the single file?
            if let Err(err) = fs::create_dir_all(&clashtui_config_dir) {
                log::error!("{}", err.to_string());
            }

            if let Err(err) = Self::first_run(&clashtui_config_dir, &names) {
                log::error!("{}", err.to_string());
            }
        }

        Self::setup_logging(&clashtui_config_dir.join("clashtui.log").to_str().unwrap());

        let clashtui_util = Rc::new(ClashTuiUtil::new(
            &clashtui_config_dir,
            &clashtui_config_dir.join("profiles"),
        ));

        let help_popup = ClashTuiListPopup::new("Help".to_string(), Rc::clone(&theme));

        let clashtui_state = Rc::new(RefCell::new(ClashTuiState::new(Rc::clone(&clashtui_util))));
        clashtui_state.borrow_mut().load_status_from_file();

        let statusbar = ClashTuiStatusBar::new(Rc::clone(&clashtui_state), Rc::clone(&theme));
        let mut app = Self {
            title: "ClashTui".to_string(),
            tabbar: ClashTuiTabBar::new(
                "".to_string(),
                vec![names.profile.clone(), names.clashsrvctl.clone()],
                Rc::clone(&theme),
            ),
            profile_tab: ProfileTab::new(
                "".to_string(),
                key_list.clone(),
                names.clone(),
                Rc::clone(&clashtui_util),
                Rc::clone(&clashtui_state),
                Rc::clone(&theme),
            ),
            clashsrvctl_tab: ClashSrvCtlTab::new(
                names.clashsrvctl.clone(),
                key_list.clone(),
                names.clone(),
                Rc::clone(&clashtui_util),
                Rc::clone(&clashtui_state),
                Rc::clone(&theme),
            ),
            should_quit: false,
            key_list,
            symbols: names,
            help_popup,
            msgpopup: MsgPopup::new(),
            statusbar,
            clashtui_state,
            clashtui_util,
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
        let mut event_state = self.help_popup.event(ev)?;

        // ## Tab Popups
        if event_state.is_notconsumed() {
            event_state = self.profile_tab.popup_event(ev)?;
        }
        if event_state.is_notconsumed() {
            event_state = self.clashsrvctl_tab.popup_event(ev)?;
        }

        Ok(event_state)
    }
    pub fn event(&mut self, ev: &Event) -> Result<EventState> {
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

            event_state = if match_key(key, &self.key_list.app_quit) {
                self.should_quit = true;
                self.clashtui_state.borrow().save_status_to_file();
                EventState::WorkDone
            } else if match_key(key, &self.key_list.app_help) {
                self.help_popup.show();
                EventState::WorkDone
            } else if match_key(key, &self.key_list.app_home_open) {
                self.clashtui_util
                    .open_dir(self.clashtui_util.clashtui_dir.as_path())?;
                EventState::WorkDone
            } else if match_key(key, &self.key_list.clash_cfg_dir_open) {
                self.clashtui_util
                    .open_dir(self.clashtui_util.clash_cfg_dir.as_path())?;
                EventState::WorkDone
            } else if match_key(key, &self.key_list.log_cat) {
                let log = self.clashtui_util.fetch_recent_logs(20);
                self.popup_list_msg(log);
                EventState::WorkDone
            } else if match_key(key, &self.key_list.clashsrvctl_restart) {
                match self.clashtui_util.clash_srv_ctl(ClashTuiOp::RestartClash) {
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
            } else if match_key(key, &self.key_list.clashsrvctl_restart_soft) {
                match self.clashtui_util.clash_client.restart() {
                    Ok(output) => {
                        let list_msg: Vec<String> =
                            output.lines().map(|line|line.trim().to_string()).collect();
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
                    event_state = self.profile_tab.event(ev)?;
                }
                if event_state.is_notconsumed() {
                    event_state = self.clashsrvctl_tab.event(ev)?;
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
                self.profile_tab.hide_msgpopup();
                if last_ev == &EventState::ProfileUpdate {
                    self.profile_tab.handle_update_profile_ev(false);
                } else if last_ev == &EventState::ProfileUpdateAll {
                    self.profile_tab.handle_update_profile_ev(true);
                }
                EventState::WorkDone
            }
            EventState::ProfileSelect => {
                self.profile_tab.hide_msgpopup();
                self.profile_tab.handle_select_profile_ev();
                EventState::WorkDone
            }
            EventState::ProfileDelete => {
                self.profile_tab.hide_msgpopup();
                self.profile_tab.handle_delete_profile_ev();
                EventState::WorkDone
            }
            EventState::EnableTun | EventState::DisableTun => {
                self.clashsrvctl_tab.hide_msgpopup();
                let res = {
                    let mut clashtui_state = self.clashtui_state.borrow_mut();
                    let tun = if last_ev == &EventState::EnableTun {
                        true
                    } else {
                        false
                    };
                    clashtui_state.set_tun(tun);
                    let profile = clashtui_state.get_profile();
                    self.clashtui_util.select_profile(profile, tun)
                };
                res.unwrap_or_else(|e| {
                    self.popup_txt_msg(e.to_string());
                });
                self.clashtui_state.borrow().save_status_to_file();
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
                self.clashtui_state
                    .borrow_mut()
                    .set_sysproxy(false);
                EventState::WorkDone
            }
            _ => EventState::NotConsumed,
        };

        ev_state
    }

    pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>) {
        let chunks = Layout::default()
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ]
                .as_ref(),
            )
            .split(f.size());

        self.update_tabbar();
        self.tabbar.draw(f, chunks[0]);

        let tabcontent_chunk = chunks[1];
        self.profile_tab.draw(f, tabcontent_chunk);
        self.clashsrvctl_tab.draw(f, tabcontent_chunk);

        self.statusbar.draw(f, chunks[2]);

        let help_area = helper::centered_percent_rect(60, 60, f.size());
        self.help_popup.draw(f, help_area);
        self.msgpopup.draw(f);
    }

    pub fn on_tick(&mut self) {}

    pub fn update_tabbar(&mut self) {
        self.profile_tab.hide();
        self.clashsrvctl_tab.hide();

        let tabname = self.tabbar.selected();
        if tabname == Some(&self.symbols.profile) {
            self.profile_tab.show();
        } else if tabname == Some(&self.symbols.clashsrvctl) {
            self.clashsrvctl_tab.show();
        }
    }

    pub fn setup_logging(log_path: &str) {
        let file_appender = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new("{d} [{l}] {t} - {m}{n}")))
            .build(log_path)
            .unwrap();

        let config = Config::builder()
            .appender(Appender::builder().build("file", Box::new(file_appender)))
            .build(
                Root::builder()
                    .appender("file")
                    .build(log::LevelFilter::Info),
            )
            .unwrap();

        log4rs::init_config(config).unwrap();
    }

    pub fn first_run(clashtui_cfg_dir: &PathBuf, symbols: &SharedSymbols) -> Result<()> {
        fs::create_dir_all(clashtui_cfg_dir.join("profiles"))?;
        fs::create_dir_all(clashtui_cfg_dir.join("templates"))?;
        fs::File::create(clashtui_cfg_dir.join("templates/template_proxy_providers"));
        fs::write(clashtui_cfg_dir.join("config.toml"), &symbols.default_clash_cfg_content)?;
        fs::write(clashtui_cfg_dir.join("basic_clash_config.yaml"), &symbols.default_basic_clash_cfg_content)?;

        Ok(())
    }
}

msgpopup_methods!(App);
