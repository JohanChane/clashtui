use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{prelude as Ra, widgets as Raw};

use crate::tui::{
    symbols::CONFIG,
    utils::Keys,
    widgets::{InputPopup, List, MsgPopup},
    EventState, SharedTheme, Visibility,
};
use crate::utils::{CfgOp, SharedClashTuiUtil};

#[derive(Visibility)]
pub struct ConfigTab {
    is_visible: bool,

    main_list: List,
    msgpopup: MsgPopup,

    clashtui_util: SharedClashTuiUtil,

    input: InputPopup,
    last_op: Option<CfgOp>,
}

impl ConfigTab {
    pub fn new(clashtui_util: SharedClashTuiUtil, theme: SharedTheme) -> Self {
        let mut operations = List::new(CONFIG.to_string(), theme);
        operations.set_items(vec![
            CfgOp::ClashConfigDir.into(),
            CfgOp::ClashConfigFile.into(),
            CfgOp::ClashCorePath.into(),
            CfgOp::ClashServiceName.into(),
        ]);

        let mut inp = InputPopup::new("Config Set".to_string());
        inp.hide();

        Self {
            is_visible: false,
            main_list: operations,
            clashtui_util,
            msgpopup: MsgPopup::new(),
            input: inp,
            last_op: None,
        }
    }

    pub fn popup_event(&mut self, ev: &Event) -> Result<EventState, ui::Infailable> {
        if !self.is_visible {
            return Ok(EventState::NotConsumed);
        }
        let mut event_state = self.msgpopup.event(ev)?;

        if event_state.is_notconsumed() {
            event_state = self.input.event(ev)?;

            if event_state == EventState::WorkDone {
                // When key is catched by input
                if let Event::Key(key) = ev {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Enter => {
                                if let Some(op) = &self.last_op {
                                    let get = self.input.get_input_data();
                                    self.clashtui_util.update_cfg(op, get);
                                    self.last_op = None;
                                }
                            }
                            KeyCode::Esc => {
                                self.last_op = None;
                            }
                            _ => {}
                        };
                    }
                }
            }
        }

        if event_state.is_notconsumed() {
            if let Event::Key(key) = ev {
                if key.kind != KeyEventKind::Press {
                    return Ok(EventState::NotConsumed);
                }
                event_state = if Keys::Select.is(key) {
                    self.last_op = Some(CfgOp::from(self.main_list.selected().unwrap().as_str()));
                    let info = self.clashtui_util.get_cfg(self.last_op.clone().unwrap());
                    self.input.set_pre_data(info);
                    self.input.show();
                    EventState::WorkDone
                } else {
                    EventState::NotConsumed
                }
            }
        }

        Ok(event_state)
    }
    pub fn event(&mut self, ev: &Event) -> Result<EventState, ui::Infailable> {
        if !self.is_visible {
            return Ok(EventState::NotConsumed);
        }

        let event_state = self.main_list.event(ev)?;

        Ok(event_state)
    }

    pub fn draw(&mut self, f: &mut Ra::Frame, area: Ra::Rect) {
        //! make config name in pop. display old config content.
        if !self.is_visible() {
            return;
        }
        use Ra::{Constraint, Layout};

        self.main_list.draw(f, area, true);

        if self.input.is_visible() {
            let input_area = Layout::default()
                .constraints([
                    Constraint::Percentage(25),
                    Constraint::Length(8),
                    Constraint::Min(0),
                ])
                .horizontal_margin(10)
                .vertical_margin(1)
                .split(f.size())[1];
            f.render_widget(Raw::Clear, input_area);

            let chunks = Layout::default()
                .constraints([Constraint::Percentage(50)])
                .margin(1)
                .split(input_area);
            self.input.draw(f, chunks[0], true);

            let block = Raw::Block::new()
                .borders(Raw::Borders::ALL)
                .border_style(Ra::Style::default().fg(Ra::Color::Rgb(135, 206, 236)))
                .title(self.main_list.selected().unwrap().clone());
            f.render_widget(block, input_area);
        }

        self.msgpopup.draw(f, area);
    }
}
