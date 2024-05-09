use super::ClashSrvOp;
use crate::msgpopup_methods;
use crate::{
    tui::{
        symbols::CLASHSRVCTL,
        tools,
        utils::Keys,
        widgets::{List, MsgPopup},
        EventState, Visibility,
    },
    utils::{SharedClashTuiState, SharedClashTuiUtil},
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

    op: Option<ClashSrvOp>,
}

impl ClashSrvCtlTab {
    pub fn new(clashtui_util: SharedClashTuiUtil, clashtui_state: SharedClashTuiState) -> Self {
        let mut operations = List::new(CLASHSRVCTL.to_string());
        operations.set_items(vec![
            ClashSrvOp::SetPermission.into(),
            ClashSrvOp::StartClashService.into(),
            ClashSrvOp::StopClashService.into(),
            ClashSrvOp::SwitchMode.into(),
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
            msgpopup: Default::default(),
            op: None,
        }
    }
}
impl super::TabEvent for ClashSrvCtlTab {
    fn popup_event(&mut self, ev: &ui::event::Event) -> Result<EventState, ui::Infailable> {
        if !self.is_visible {
            return Ok(EventState::NotConsumed);
        }
        let event_state;
        if self.mode_selector.is_visible() {
            event_state = self.mode_selector.event(ev)?;
            if event_state == EventState::WorkDone {
                return Ok(event_state);
            }
            if let ui::event::Event::Key(key) = ev {
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
    fn event(&mut self, ev: &ui::event::Event) -> Result<EventState, ui::Infailable> {
        if !self.is_visible {
            return Ok(EventState::NotConsumed);
        }

        let event_state;
        if let ui::event::Event::Key(key) = ev {
            if key.kind != ui::event::KeyEventKind::Press {
                return Ok(EventState::NotConsumed);
            }
            // override `Enter`
            event_state = if &Keys::Select == key {
                let op = ClashSrvOp::from(self.main_list.selected().unwrap().as_str());
                if let ClashSrvOp::SwitchMode = op {
                    self.mode_selector.show();
                } else {
                    self.op.replace(op);
                    self.popup_txt_msg("Working...".to_string());
                }
                EventState::WorkDone
            } else {
                self.main_list.event(ev)?
            };
        } else {
            event_state = EventState::NotConsumed
        }

        Ok(event_state)
    }
    fn late_event(&mut self) {
        if let Some(op) = self.op.take() {
            self.hide_msgpopup();
            match op {
                ClashSrvOp::SwitchMode => unreachable!(),
                _ => match self.clashtui_util.clash_srv_ctl(op.clone()) {
                    Ok(output) => {
                        self.popup_list_msg(output.lines().map(|line| line.trim().to_string()));
                    }
                    Err(err) => {
                        self.popup_txt_msg(err.to_string());
                    }
                },
            }
            match op {
                // Ops that doesn't need refresh
                ClashSrvOp::SetPermission => {},

                ClashSrvOp::StartClashService => {
                    std::thread::sleep(std::time::Duration::from_secs(2));      // Waiting for mihomo to finish starting.
                    self.clashtui_state.borrow_mut().refresh();
                }
                _ => {
                    self.clashtui_state.borrow_mut().refresh();
                },
            }
        }
    }
    fn draw(&mut self, f: &mut ratatui::prelude::Frame, area: ratatui::prelude::Rect) {
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
