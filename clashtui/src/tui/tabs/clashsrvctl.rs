use crossterm::event::{Event, KeyEventKind};
use ratatui::prelude as Ra;

use crate::msgpopup_methods;
use crate::{
    tui::{
        symbols::CLASHSRVCTL,
        tools,
        utils::Keys,
        widgets::{List, MsgPopup},
        EventState, Visibility,
    },
    utils::{ClashSrvOp, SharedClashTuiState, SharedClashTuiUtil},
};
use api::Mode;

#[derive(Visibility)]
pub struct ClashSrvCtlTab {
    is_visible: bool,

    main_list: List,
    msgpopup: MsgPopup,

    mode_selector: List,

    clashtui_util: SharedClashTuiUtil,
    clashtui_state: SharedClashTuiState,
}

impl ClashSrvCtlTab {
    pub fn new(clashtui_util: SharedClashTuiUtil, clashtui_state: SharedClashTuiState) -> Self {
        let title = CLASHSRVCTL.to_string();
        let mut operations = List::new(title);
        operations.set_items(vec![
            #[cfg(target_os = "linux")]
            ClashSrvOp::SetPermission.into(),
            ClashSrvOp::StartClashService.into(),
            ClashSrvOp::StopClashService.into(),
            ClashSrvOp::TestClashConfig.into(),
            ClashSrvOp::UpdateGeoData.into(),
            ClashSrvOp::SwitchMode.into(),
            #[cfg(target_os = "windows")]
            ClashSrvOp::SwitchSysProxy.into(),
            #[cfg(target_os = "windows")]
            ClashSrvOp::EnableLoopback.into(),
            #[cfg(target_os = "windows")]
            ClashSrvOp::InstallSrv.into(),
            #[cfg(target_os = "windows")]
            ClashSrvOp::UnInstallSrv.into(),
        ]);
        let mut modes = List::new("Mode".to_string());
        modes.set_items(vec![
            Mode::Rule.into(),
            Mode::Direct.into(),
            Mode::Global.into(),
        ]);
        modes.hide();

        Self {
            is_visible: false,
            main_list: operations,
            mode_selector: modes,
            clashtui_util,
            clashtui_state,
            msgpopup: MsgPopup::new(),
        }
    }

    pub fn popup_event(&mut self, ev: &Event) -> Result<EventState, ui::Infailable> {
        if !self.is_visible {
            return Ok(EventState::NotConsumed);
        }
        let event_state;
        if self.mode_selector.is_visible() {
            event_state = self.mode_selector.event(ev)?;
            if event_state == EventState::WorkDone {
                return Ok(event_state);
            }
            if let Event::Key(key) = ev {
                if &Keys::Select == key {
                    if let Some(new) = self.mode_selector.selected() {
                        self.clashtui_state.borrow_mut().set_mode(new.clone());
                    }
                    self.mode_selector.hide();
                }
                if &Keys::Esc == key {
                    self.mode_selector.hide();
                }
            }
            return Ok(EventState::WorkDone);
        }

        event_state = self.msgpopup.event(ev)?;

        Ok(event_state)
    }
    pub fn event(&mut self, ev: &Event) -> Result<EventState, ui::Infailable> {
        if !self.is_visible {
            return Ok(EventState::NotConsumed);
        }

        let mut event_state;
        if let Event::Key(key) = ev {
            if key.kind != KeyEventKind::Press {
                return Ok(EventState::NotConsumed);
            }

            event_state = if &Keys::Select == key {
                let op_str = self.main_list.selected().unwrap();
                let op = ClashSrvOp::from(op_str.as_str());
                match op {
                    #[cfg(target_os = "windows")]
                    ClashSrvOp::SwitchSysProxy => {
                        self.popup_txt_msg("SwitchSysProxy...".to_string());
                        EventState::SwitchSysProxy
                    }
                    ClashSrvOp::SwitchMode => {
                        self.mode_selector.show();
                        EventState::WorkDone
                    }
                    _ => {
                        match self.clashtui_util.clash_srv_ctl(op) {
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
                }
            } else {
                EventState::NotConsumed
            };

            if event_state == EventState::NotConsumed {
                event_state = self.main_list.event(ev)?;
            }
        } else {
            event_state = EventState::NotConsumed
        }

        Ok(event_state)
    }

    pub fn draw(&mut self, f: &mut Ra::Frame, area: Ra::Rect) {
        if !self.is_visible() {
            return;
        }

        self.main_list.draw(f, area, true);
        if self.mode_selector.is_visible() {
            let select_area = tools::centered_percent_rect(60, 30, f.size());
            f.render_widget(ratatui::widgets::Clear, select_area);
            self.mode_selector.draw(f, select_area, true);
        }
        self.msgpopup.draw(f, area);
    }
}

msgpopup_methods!(ClashSrvCtlTab);
