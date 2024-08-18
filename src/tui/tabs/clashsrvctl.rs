use crate::backend::api::Mode;
use crate::utils::ClashSrvOp;
use crate::{msgpopup_methods, ui};
use crate::{
    tui::{
        symbols::CLASHSRVCTL,
        tools,
        utils::Keys,
        widgets::{List, MsgPopup},
        EventState, Visibility,
    },
    utils::{SharedBackend, SharedState},
};

pub struct ClashSrvCtlTab {
    is_visible: bool,

    main_list: List,
    msgpopup: MsgPopup,

    mode_selector: List,

    util: SharedBackend,
    state: SharedState,

    op: Option<ClashSrvOp>,
}
impl Visibility for ClashSrvCtlTab {
    fn is_visible(&self) -> bool {
        self.is_visible
    }

    fn show(&mut self) {
        self.is_visible = true;
    }

    fn hide(&mut self) {
        self.is_visible = false;
    }

    fn set_visible(&mut self, b: bool) {
        self.is_visible = b;
    }
}
impl ClashSrvCtlTab {
    pub fn new(util: SharedBackend, state: SharedState) -> Self {
        let mut operations = List::new(CLASHSRVCTL.to_string());
        operations.set_items(vec![
            #[cfg(target_os = "linux")]
            ClashSrvOp::SetPermission.into(),
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            ClashSrvOp::StartClashService.into(),
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            ClashSrvOp::StopClashService.into(),
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
            util,
            state,
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
                        self.state.borrow_mut().set_mode(new.clone());
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
                #[allow(irrefutable_let_patterns)]
                // currently, only [`SwitchMode`] is impl on macos
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
                #[cfg(target_os = "windows")]
                ClashSrvOp::SwitchSysProxy => {
                    let cur = self.state.borrow().get_sysproxy().map_or(true, |b| !b);
                    self.state.borrow_mut().set_sysproxy(cur);
                    self.hide_msgpopup();
                }
                #[allow(unreachable_patterns)] // currently, only [`SwitchMode`] is impl on macos
                _ => match self.util.clash_srv_ctl(op) {
                    Ok(output) => {
                        self.popup_list_msg(output.lines().map(|line| line.trim().to_string()));
                    }
                    Err(err) => {
                        self.popup_txt_msg(err.to_string());
                    }
                },
            }
            self.state.borrow_mut().refresh();
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
