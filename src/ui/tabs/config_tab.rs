use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{prelude as Ra, widgets as Raw};

use super::CommonTab;
use crate::ui::{
    popups::{ClashTuiInputPopup, MsgPopup},
    utils::{ClashTuiList, SharedTheme},
    EventState, SharedSymbols,
};
use crate::utils::ConfigOp;
use crate::utils::SharedClashTuiUtil;
use crate::{msgpopup_methods, title_methods, visible_methods};

pub struct ConfigTab {
    title: String,
    is_visible: bool,

    setting_list: ClashTuiList,
    msgpopup: MsgPopup,

    clashtui_util: SharedClashTuiUtil,

    input: ClashTuiInputPopup,
    last_op: Option<ConfigOp>,
}

impl ConfigTab {
    pub fn new(
        symbols: SharedSymbols,
        clashtui_util: SharedClashTuiUtil,

        theme: SharedTheme,
    ) -> Self {
        let mut operations = ClashTuiList::new(symbols.config.clone(), theme);
        operations.set_items(vec![
            ConfigOp::ClashConfigDir.into(),
            ConfigOp::ClashConfigFile.into(),
            ConfigOp::ClashCorePath.into(),
            ConfigOp::ClashServiceName.into(),
        ]);

        let mut inp = ClashTuiInputPopup::new("Config Set".to_string());
        inp.hide();

        Self {
            title: symbols.config.clone(),
            is_visible: false,
            setting_list: operations,
            clashtui_util,
            msgpopup: MsgPopup::new(),
            input: inp,
            last_op: None,
        }
    }

    pub fn popup_event(&mut self, ev: &Event) -> Result<EventState, ()> {
        if !self.is_visible {
            return Ok(EventState::NotConsumed);
        }

        let event_state = self.msgpopup.event(ev).unwrap();

        Ok(event_state)
    }
}

impl CommonTab for ConfigTab {
    fn event(&mut self, ev: &Event) -> Result<EventState, ()> {
        if !self.is_visible {
            return Ok(EventState::NotConsumed);
        }

        let mut event_state = self.setting_list.event(ev).unwrap();
        if event_state.is_consumed() {
            return Ok(event_state);
        }

        event_state = self.input.event(ev).unwrap();
        if event_state.is_consumed() {
            return Ok(event_state);
        }

        if self.last_op.is_none() {
            self.last_op = Some(ConfigOp::from(
                self.setting_list.selected().unwrap().as_str(),
            ));
            log::debug!("Action: {:?}", self.last_op);
            self.input.show();
        }

        if let Event::Key(key) = ev {
            if key.kind == KeyEventKind::Press {
                event_state = match key.code {
                    KeyCode::Enter => {
                        if let Some(op) = &self.last_op {
                            self.input.handle_enter_ev();
                            self.clashtui_util
                                .update_config(op, self.input.get_input_data());
                            self.hide();
                            self.last_op = None;
                        }

                        EventState::WorkDone
                    }
                    KeyCode::Esc => {
                        self.input.handle_esc_ev();
                        self.last_op = None;
                        self.hide();

                        EventState::WorkDone
                    }
                    _ => {
                        event_state = self.input.event(ev).unwrap();
                        event_state
                    }
                };
            }
        }

        Ok(event_state)
    }

    fn draw<B: Ra::Backend>(&mut self, f: &mut Ra::Frame<B>, area: Ra::Rect) {
        //! make config name in pop. display old config content.
        if !self.is_visible() {
            return;
        }
        use Ra::{Constraint, Layout};

        self.setting_list.draw(f, area);

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
            self.input.draw(f, chunks[0]);

            let block = Raw::Block::new()
                .borders(Raw::Borders::ALL)
                .border_style(Ra::Style::default().fg(Ra::Color::Rgb(135, 206, 236)))
                .title("InputProfile");
            f.render_widget(block, input_area);
        }

        self.msgpopup.draw(f, area);
    }
}

title_methods!(ConfigTab);
visible_methods!(ConfigTab);
msgpopup_methods!(ConfigTab);
