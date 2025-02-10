use crate::backend::ServiceOp;
use crate::define_enum;
use crate::tui::widget::Browser;
use crate::{clash::webapi::Mode, tui::widget::PopRes};
use crossterm::event::KeyEvent;

use crate::tui::{frontend::consts::TAB_TITLE_SERVICE, widget::List, Drawable, EventState};
use ratatui::prelude as Ra;
use Ra::{Frame, Rect};

use super::{Call, CallBack, PopMsg, TabCont};

#[derive(Debug)]
pub enum BackendOp {
    SwitchMode(Mode),
    ServiceCTL(ServiceOp),
    TuiExtend(ExtendOp),
    OpenThis(std::path::PathBuf),
}

define_enum!(
    #[derive(Clone, Copy, Debug)]
    pub enum ExtendOp {
        ViewClashtuiConfigDir,
        // Generate a list of information
        // about the application and clash core
        GenerateInfoList,
        FullLog,
    }
);

pub(in crate::tui::frontend) struct ServiceTab {
    inner: List,
    popup_content: Option<PopMsg>,
    backend_content: Option<Call>,
    file_browser: Option<Browser>,
}

const MODE: [Mode; 3] = [Mode::Rule, Mode::Direct, Mode::Global];

impl Default for ServiceTab {
    fn default() -> Self {
        let mut inner = List::new(TAB_TITLE_SERVICE.to_string());
        let items = inner.get_items_mut();
        items.push("SwitchMode".to_owned());
        items.extend(ServiceOp::ALL.into_iter().map(|v| v.to_string()));

        items.push("-----".to_owned());
        items.extend(ExtendOp::ALL.into_iter().map(|v| v.to_string()));

        Self {
            inner,
            popup_content: None,
            backend_content: None,
            file_browser: None,
        }
    }
}

impl std::fmt::Display for ServiceTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", crate::tui::frontend::consts::TAB_TITLE_SERVICE)
    }
}

impl TabCont for ServiceTab {
    fn get_backend_call(&mut self) -> Option<Call> {
        self.backend_content.take()
    }

    fn get_popup_content(&mut self) -> Option<PopMsg> {
        self.popup_content.take()
    }

    fn apply_backend_call(&mut self, op: CallBack) {
        if let CallBack::ServiceCTL(result) = op {
            let result = ["Success"]
                .into_iter()
                .chain(result.lines())
                .map(|s| s.to_owned())
                .collect();
            self.popup_content.replace(PopMsg::Prompt(result));
        } else if let CallBack::TuiExtend(result) = op {
            self.popup_content.replace(PopMsg::Prompt(result));
        } else {
            unreachable!("{} get unexpected op", TAB_TITLE_SERVICE)
        }
    }
    // this tab just display info but don't ask
    fn apply_popup_result(&mut self, res: PopRes) -> EventState {
        match res {
            PopRes::Selected(idx) => {
                let mode = MODE[idx];
                let pak = Call::Service(BackendOp::SwitchMode(mode));
                self.backend_content.replace(pak);
                let msg = PopMsg::Prompt(vec!["Working".to_owned()]);
                self.popup_content.replace(msg);
            }
            PopRes::Choices(_) | PopRes::Input(_) => unreachable!(),
        }
        EventState::WorkDone
    }
}

impl Drawable for ServiceTab {
    fn render(&mut self, f: &mut Frame, area: Rect, _: bool) {
        self.inner.render(f, area, self.file_browser.is_none());
        if let Some(instance) = self.file_browser.as_mut() {
            instance.render(f, area, true);
        }
    }
    /// - Catched event -> [EventState::WorkDone]
    /// - unrecognized event -> [EventState::NotConsumed]
    fn handle_key_event(&mut self, ev: &KeyEvent) -> EventState {
        if let Some(instance) = self.file_browser.as_mut() {
            match instance.handle_key_event(ev) {
                EventState::Yes => {
                    self.backend_content = Some(Call::Service(BackendOp::OpenThis(
                        self.file_browser.take().unwrap().path(),
                    )))
                }
                EventState::Cancel => self.file_browser = None,
                EventState::NotConsumed | EventState::WorkDone => (),
            }
            return EventState::WorkDone;
        };
        let event_state = self.inner.handle_key_event(ev);
        match event_state {
            EventState::Yes => {
                if let Some(index) = self.inner.selected() {
                    match index {
                        0 => {
                            self.popup_content = Some(PopMsg::SelectList(
                                "Mode".to_owned(),
                                Vec::from_iter(MODE.into_iter().map(|v| v.into())),
                            ));
                        }
                        idx if idx < ServiceOp::const_len() + 1 => {
                            let op = ServiceOp::ALL[index - 1];
                            self.backend_content = Some(Call::Service(BackendOp::ServiceCTL(op)));
                            self.popup_content = Some(PopMsg::Prompt(vec!["working".to_owned()]));
                        }
                        idx if idx == ServiceOp::const_len() + 1 => (),
                        idx if idx < ServiceOp::const_len() + 2 + ExtendOp::const_len() => {
                            let op = ExtendOp::ALL[index - 2 - ServiceOp::const_len()];
                            if let ExtendOp::ViewClashtuiConfigDir = op {
                                self.file_browser = Some(Browser::new(&crate::HOME_DIR))
                            } else {
                                self.backend_content =
                                    Some(Call::Service(BackendOp::TuiExtend(op)));
                                self.popup_content =
                                    Some(PopMsg::Prompt(vec!["working".to_owned()]));
                            }
                        }
                        _ => unreachable!(),
                    }
                };
            }
            EventState::Cancel | EventState::NotConsumed | EventState::WorkDone => (),
        }
        event_state.unify()
    }
}
