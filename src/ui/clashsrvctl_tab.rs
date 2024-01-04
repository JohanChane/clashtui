use anyhow::Result;
use crossterm::event::{Event, KeyEventKind};
use ratatui::prelude::*;

use crate::clashtui_state::SharedClashTuiState;
use crate::keys::{match_key, SharedKeyList};
use crate::ui::widgets::SharedTheme;
use crate::ui::ClashTuiOp;
use crate::ui::SharedSymbols;
use crate::ui::{widgets::ClashTuiList, EventState, MsgPopup};
use crate::utils::SharedClashTuiUtil;
use crate::{msgpopup_methods, title_methods, visible_methods};

pub struct ClashSrvCtlTab {
    title: String,
    is_visible: bool,

    srvctl_list: ClashTuiList,
    msgpopup: MsgPopup,

    key_list: SharedKeyList,
    symbols: SharedSymbols,

    clashtui_util: SharedClashTuiUtil,
    clashtui_state: SharedClashTuiState,
}

impl ClashSrvCtlTab {
    pub fn new(
        title: String,
        key_list: SharedKeyList,
        symbols: SharedSymbols,
        clashtui_util: SharedClashTuiUtil,
        clashtui_state: SharedClashTuiState,

        theme: SharedTheme,
    ) -> Self {
        let mut operations = ClashTuiList::new(symbols.clashsrvctl.clone(), theme);
        operations.set_items(vec![
            ClashTuiOp::TestClashConfig.into(),
            ClashTuiOp::EnableTun.into(),
            ClashTuiOp::DisableTun.into(),
            #[cfg(target_os = "windows")]
            ClashTuiOp::EnableSysProxy.into(),
            #[cfg(target_os = "windows")]
            ClashTuiOp::DisableSysProxy.into(),
            #[cfg(target_os = "windows")]
            ClashTuiOp::EnableLoopback.into(),
            #[cfg(target_os = "windows")]
            ClashTuiOp::InstallSrv.into(),
            #[cfg(target_os = "windows")]
            ClashTuiOp::UnInstallSrv.into(),
        ]);

        Self {
            title,
            is_visible: false,
            srvctl_list: operations,
            key_list,
            symbols,
            clashtui_util,
            clashtui_state,
            msgpopup: MsgPopup::new(),
        }
    }

    pub fn popup_event(&mut self, ev: &Event) -> Result<EventState> {
        if !self.is_visible {
            return Ok(EventState::NotConsumed);
        }

        let mut event_state = self.msgpopup.event(ev)?;

        Ok(event_state)
    }

    pub fn event(&mut self, ev: &Event) -> Result<EventState> {
        if !self.is_visible {
            return Ok(EventState::NotConsumed);
        }

        let mut event_state = EventState::NotConsumed;
        if let Event::Key(key) = ev {
            if key.kind != KeyEventKind::Press {
                return Ok(EventState::NotConsumed);
            }

            event_state = if match_key(key, &self.key_list.clashsrvctl_select) {
                let op_str = self.srvctl_list.selected().unwrap();
                let op: ClashTuiOp = ClashTuiOp::from(op_str.as_ref());
                match op {
                    ClashTuiOp::EnableTun => {
                        self.popup_txt_msg("EnableTun...".to_string());
                        EventState::EnableTun
                    }
                    ClashTuiOp::DisableTun => {
                        self.popup_txt_msg("DisableTun...".to_string());
                        EventState::DisableTun
                    }
                    #[cfg(target_os = "windows")]
                    ClashTuiOp::EnableSysProxy => {
                        self.popup_txt_msg("EnableSysProxy...".to_string());
                        EventState::EnableSysProxy
                    }
                    #[cfg(target_os = "windows")]
                    ClashTuiOp::DisableSysProxy => {
                        self.popup_txt_msg("DisableSysProxy...".to_string());
                        EventState::DisableSysProxy
                    }
                    _ => {
                        let res = self.clashtui_util.borrow().clash_srv_ctl(op);
                        match res {
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
                event_state = self.srvctl_list.event(ev)?;
            }
        }

        Ok(event_state)
    }

    pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        if !self.is_visible() {
            return;
        }

        self.srvctl_list.draw(f, area);
        self.msgpopup.draw(f);
    }
}

title_methods!(ClashSrvCtlTab);
visible_methods!(ClashSrvCtlTab);
msgpopup_methods!(ClashSrvCtlTab);
