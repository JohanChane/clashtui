use crate::backend::{Mode, ServiceOp};
use crate::tui::widget::{Browser, List, Path, PopRes, Popmsg};
use crate::tui::{Drawable, EventState, frontend::consts::TAB_TITLE_SERVICE};

use Ra::{Frame, Rect};
use crossterm::event::KeyEvent;
use ratatui::prelude as Ra;
use strum::{EnumCount, VariantArray};

use super::{Call, CallBack, PopMsg, TabCont};

#[derive(Debug)]
pub enum BackendOp {
    SwitchMode(Mode),
    ServiceCTL(ServiceOp),
    TuiExtend(ExtendOp),
    OpenThis(std::path::PathBuf),
    Preview(std::path::PathBuf),
}

#[derive(Clone, Copy, Debug, EnumCount, VariantArray, strum::Display)]
pub enum ExtendOp {
    /// won't pass to backend
    ViewClashtuiConfigDir,
    // Generate a list of information
    // about the application and clash core
    GenerateInfoList,
    FullLog,
}

pub(in crate::tui::frontend) struct ServiceTab {
    inner: List,
    popup_content: Option<PopMsg>,
    backend_content: Option<Call>,
    file_browser: Option<Browser>,
}

impl Default for ServiceTab {
    fn default() -> Self {
        let mut inner = List::new(TAB_TITLE_SERVICE);
        let items = inner.get_items_mut();
        items.push(Modes.to_string());
        items.extend(ServiceOp::VARIANTS.iter().map(|v| v.to_string()));

        items.push("-----".to_owned());
        items.extend(ExtendOp::VARIANTS.iter().map(|v| v.to_string()));

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
            self.popup_content
                .replace(PopMsg::msg(format!("Success\n{}", result)));
        } else if let CallBack::TuiExtend(result) = op {
            self.popup_content.replace(PopMsg::msg(result));
        } else {
            unreachable!("{} get unexpected op", TAB_TITLE_SERVICE)
        }
    }
    fn apply_popup_result(&mut self, _: PopRes) {
        unreachable!()
    }
}

impl Drawable for ServiceTab {
    fn render(&mut self, f: &mut Frame, area: Rect, _: bool) {
        self.inner.render(f, area, self.file_browser.is_none());
        if let Some(instance) = self.file_browser.as_mut() {
            instance.render(f, area, true);
        }
    }
    fn handle_key_event(&mut self, ev: &KeyEvent) -> EventState {
        if let Some(instance) = self.file_browser.as_mut() {
            match instance.handle_key_event(ev) {
                EventState::Yes => {
                    self.backend_content = Some(Call::Service(
                        match self.file_browser.take().unwrap().path() {
                            Path::Open(path) => BackendOp::OpenThis(path),
                            Path::Preview(path) => BackendOp::Preview(path),
                        },
                    ))
                }
                EventState::Cancel => self.file_browser = None,
                EventState::NotConsumed | EventState::Consumed => (),
            }
            return EventState::Consumed;
        };
        match self.inner.handle_key_event(ev) {
            EventState::Yes => {
                let Some(index) = self.inner.selected() else {
                    return EventState::NotConsumed;
                };

                const SERVICE_OP: usize = ServiceOp::COUNT;
                const SEP: usize = ServiceOp::COUNT + 1;
                const EXTEND_OP_START: usize = ServiceOp::COUNT + 2;
                const EXTEND_OP_END: usize = ServiceOp::COUNT + 1 + ExtendOp::COUNT;

                match index {
                    0 => self.popup_content = Some(PopMsg::new(Modes)),
                    1..=SERVICE_OP => {
                        let op = ServiceOp::VARIANTS[index - 1];
                        self.backend_content = Some(Call::Service(BackendOp::ServiceCTL(op)));
                        self.popup_content = Some(PopMsg::working());
                    }
                    SEP => (),
                    EXTEND_OP_START..=EXTEND_OP_END => {
                        let op = ExtendOp::VARIANTS[index - 2 - ServiceOp::COUNT];
                        if let ExtendOp::ViewClashtuiConfigDir = op {
                            self.file_browser = Some(Browser::new(crate::DataDir::get()))
                        } else {
                            self.backend_content = Some(Call::Service(BackendOp::TuiExtend(op)));
                            self.popup_content = Some(PopMsg::working());
                        }
                    }
                    _ => unreachable!(),
                }
            }
            EventState::Cancel | EventState::Consumed => (),
            EventState::NotConsumed => return EventState::NotConsumed,
        }
        EventState::Consumed
    }
}

struct Modes;

impl std::fmt::Display for Modes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SwitchMode")
    }
}

impl Popmsg for Modes {
    fn start(&self, pop: &mut crate::tui::widget::Popup) {
        pop.start()
            .clear_all()
            .set_title("Mode")
            .set_choices(Mode::VARIANTS.iter().map(|v| v.to_string()));
    }

    fn next(
        self: Box<Self>,
        pop: &mut crate::tui::widget::Popup,
    ) -> crate::tui::widget::PopupState {
        let Some(PopRes::Selected(idx)) = pop.collect() else {
            unreachable!()
        };
        let mode = Mode::VARIANTS[idx];
        crate::tui::widget::PopupState::ToBackend(Call::Service(BackendOp::SwitchMode(mode)))
    }
}
