use crate::utils::ServiceOp;
use crate::{clash::webapi::Mode, tui::widget::PopRes};
use crossterm::event::KeyEvent;

use crate::{
    tui::{
        frontend::consts::TAB_TITLE_SERVICE,
        widget::{tools, List},
        Drawable, EventState,
    },
    utils::CallBack,
};
use ratatui::prelude as Ra;
use ratatui::widgets as Raw;
use Ra::{Frame, Rect};

use super::{Call, PopMsg, TabCont};

pub enum BackendOp {
    SwitchMode(Mode),
    ServiceCTL(ServiceOp),
}

pub(in crate::tui::frontend) struct ServiceTab {
    inner: List,
    mode_selector: List,
    select_mode: bool,
    popup_content: Option<PopMsg>,
    backend_content: Option<Call>,
}

const MODE: [Mode; 3] = [Mode::Rule, Mode::Direct, Mode::Global];

impl ServiceTab {
    /// Creates a new [`ServiceTab`].
    pub fn new() -> Self {
        let mut operations = List::new(TAB_TITLE_SERVICE.to_string());
        let mut inner_items = vec!["SwitchMode".to_owned()];
        inner_items.extend(ServiceOp::ALL.into_iter().map(|v| v.into()));
        operations.set_items(inner_items);

        let mut modes = List::new("Mode".to_owned());
        let mode_items = Vec::from_iter(MODE.into_iter().map(|v| v.into()));
        modes.set_items(mode_items);

        Self {
            inner: operations,
            mode_selector: modes,
            select_mode: false,
            popup_content: None,
            backend_content: None,
        }
    }
}

impl Default for ServiceTab {
    fn default() -> Self {
        Self::new()
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
        } else {
            unreachable!("{} get unexpected op", TAB_TITLE_SERVICE)
        }
    }
    // this tab just display info but don't ask
    fn apply_popup_result(&mut self, _res: PopRes) -> EventState {
        unreachable!()
    }
}

impl Drawable for ServiceTab {
    fn render(&mut self, f: &mut Frame, area: Rect, _: bool) {
        self.inner.render(f, area, !self.select_mode);
        if self.select_mode {
            let select_area = tools::centered_rect(
                Ra::Constraint::Percentage(60),
                Ra::Constraint::Percentage(30),
                f.area(),
            );
            f.render_widget(Raw::Clear, select_area);
            self.mode_selector.render(f, select_area, true);
        }
    }
    // call [`TabCont::apply_popup_result`] first
    fn handle_key_event(&mut self, ev: &KeyEvent) -> EventState {
        let event_state;
        if self.select_mode {
            // ## handle mode_selector
            event_state = self.mode_selector.handle_key_event(ev);
            match event_state {
                EventState::Yes => {
                    if let Some(mode_index) = self.mode_selector.selected() {
                        let mode = MODE[mode_index];
                        let pak = Call::Service(BackendOp::SwitchMode(mode));
                        self.backend_content.replace(pak);
                        let msg = PopMsg::Prompt(vec!["Working".to_owned()]);
                        self.popup_content.replace(msg);
                    };
                    self.select_mode = false;
                }
                EventState::Cancel => self.select_mode = false,
                EventState::NotConsumed | EventState::WorkDone => (),
            }
        } else {
            // ## handle inner
            event_state = self.inner.handle_key_event(ev);
            match event_state {
                EventState::Yes => {
                    if let Some(index) = self.inner.selected() {
                        if index == 0 {
                            self.select_mode = true;
                        } else {
                            let op = ServiceOp::ALL[index - 1];
                            let pak = Call::Service(BackendOp::ServiceCTL(op));
                            self.backend_content.replace(pak);
                        }
                    };
                }
                EventState::Cancel | EventState::NotConsumed | EventState::WorkDone => (),
            }
        }
        event_state.unify()
    }
}
