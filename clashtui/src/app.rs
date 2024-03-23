use core::cell::{OnceCell, RefCell};
use std::{path::PathBuf, rc::Rc};
use std::io::{Write, BufRead, Read};

use ui::event;

use crate::{msgpopup_methods, utils};
use crate::tui::{
    tabs::{ClashSrvCtlTab, ProfileTab, TabEvent, Tabs},
    tools,
    utils::{HelpPopUp, InfoPopUp, Keys},
    widgets::MsgPopup,
    EventState, StatusBar, TabBar, Theme, Visibility,
};
use crate::utils::{
    CfgError, ClashTuiUtil, Flag, Flags, SharedClashTuiState, SharedClashTuiUtil, State,
};

/// Mihomo (Clash.Meta) TUI Client
///
/// A tui tool for mihomo
#[derive(argh::FromArgs)]
pub struct CliEnv {
    /// don't show UI but only update all profiles
    #[argh(switch, short = 'u')]
    pub update_all_profiles: bool,
    /// print version information and exit
    #[argh(switch, short = 'v')]
    pub version: bool,
}

pub struct App {
    tabbar: TabBar,
    tabs: Vec<Tabs>,
    pub should_quit: bool,
    help_popup: OnceCell<Box<HelpPopUp>>,
    info_popup: InfoPopUp,
    msgpopup: MsgPopup,

    pub clashtui_util: SharedClashTuiUtil,
    statusbar: StatusBar,
}

impl App {
    pub fn new(
        flags: &Flags<Flag>,
        clashtui_config_dir: &PathBuf,
    ) -> (Option<Self>, Vec<CfgError>) {
        let (util, err_track) =
            ClashTuiUtil::new(clashtui_config_dir, !flags.contains(Flag::FirstInit));
        let clashtui_util = SharedClashTuiUtil::new(util);

        let clashtui_state =
            SharedClashTuiState::new(RefCell::new(State::new(Rc::clone(&clashtui_util))));
        let _ = Theme::load(None).map_err(|e| log::error!("Loading Theme:{e}"));

        let tabs: Vec<Tabs> = vec![
            Tabs::Profile(ProfileTab::new(
                clashtui_util.clone(),
                clashtui_state.clone(),
            )),
            Tabs::ClashSrvCtl(ClashSrvCtlTab::new(
                clashtui_util.clone(),
                clashtui_state.clone(),
            )),
        ]; // Init the tabs
        let tabbar = TabBar::new(tabs.iter().map(|v| v.to_string()).collect());
        let statusbar = StatusBar::new(Rc::clone(&clashtui_state));
        let info_popup = InfoPopUp::with_items(&clashtui_util.clash_version());

        let mut app = Self {
            tabbar,
            should_quit: false,
            help_popup: Default::default(),
            info_popup,
            msgpopup: Default::default(),
            statusbar,
            clashtui_util,
            tabs,
        };

        app.do_some_job_after_initapp_before_setupui();
        
        (Some(app), err_track)
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
                        .clashtui_util
                        .open_dir(&PathBuf::from(&self.clashtui_util.tui_cfg.clash_cfg_dir))
                        .map_err(|e| log::error!("ODIR: {}", e));
                    EventState::WorkDone
                }
                Keys::AppConfig => {
                    let _ = self
                        .clashtui_util
                        .open_dir(self.clashtui_util.clashtui_dir.as_path())
                        .map_err(|e| log::error!("ODIR: {}", e));
                    EventState::WorkDone
                }
                Keys::LogCat => {
                    let log = self.clashtui_util.fetch_recent_logs(20);
                    self.popup_list_msg(log);
                    EventState::WorkDone
                }
                Keys::SoftRestart => {
                    match self.clashtui_util.restart_clash() {
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
        self.clashtui_util
            .tui_cfg
            .to_file(config_path)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    fn do_some_job_after_initapp_before_setupui(&mut self) {
        // ## Correct the perm of files in clash_cfg_dir.
        if ! self.clashtui_util.check_perms_of_ccd_files() {
            let ccd_str = self.clashtui_util.tui_cfg.clash_cfg_dir.as_str();
            if ! utils::is_run_as_root() {
                print!("The permissions of the '{}' files are incorrect. clashtui need to run as root to correct. Proceed with running as root? [Y/n] ", ccd_str);
                std::io::stdout().flush().expect("Failed to flush stdout");

                let mut input = String::new();
                let stdin = std::io::stdin();
                stdin.lock().read_line(&mut input).unwrap();

                if input.trim().to_lowercase().as_str() == "y" {
                    utils::run_as_root();
                }

            } else {
                if utils::is_clashtui_ep() {
                    println!("\nStart correct the permissions of files in '{}':\n", ccd_str);
                    let dir = std::path::Path::new(ccd_str);
                    if let Some(group_name) = utils::get_file_group_name(&dir.to_path_buf()) {
                        utils::restore_fileop_as_root();
                        utils::modify_file_perms_in_dir(&dir.to_path_buf(), group_name.as_str());
                    }
                    print!("\nEnd correct the permissions of files in '{}'. \n\nPrepare to restart clashtui. Press any key to continue. ", ccd_str);
                    std::io::stdout().flush().expect("Failed to flush stdout");
                    let _ = std::io::stdin().read(&mut [0u8]);

                    utils::run_as_previous_user();      // 
                } else {      // user manually executing `sudo clashtui`
                    // Do nothing, as root is unaffected by permissions.
                }
            }
        }

        let cli_env: CliEnv = argh::from_env();

        // ## CliMode
        let mut is_cli_mode = false;

        // ToDo: Check MD5s(profile, proxy-providers) of the current profile. If they are changed, reload the profile.
        //self.clashtui_util.tui_cfg.current_profile;
        if cli_env.update_all_profiles {
            is_cli_mode = true;

            log::info!("Cron Mode!");
            self.clashtui_util.get_profile_names()
                .unwrap()
                .into_iter()
                .inspect(|s| println!("\nProfile: {s}"))
                .filter_map(|v| {
                    self.clashtui_util.update_profile(&v, false)
                        .map_err(|e| println!("- Error! {e}"))
                        .ok()
                })
                .flatten()
                .for_each(|s| println!("- {s}"));
        }

        if is_cli_mode {
            std::process::exit(0);
        }
    }
}

msgpopup_methods!(App);
