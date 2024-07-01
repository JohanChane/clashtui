use ratatui::widgets::TableState;
use ui::widgets::ConfirmPopup;
use ui::{EventState, Visibility};

use crate::{msgpopup_methods, tui::utils::Keys, utils::SharedBackend};
crate::utils::define_enum!(
    #[derive(PartialEq, Eq)]
    Cop,
    [
        Refresh,
        PreTerminate,
        Terminate,
        ShowInfo,
        Indialog,
        PreTerminateAll,
        TerminateAll
    ]
);
#[derive(Visibility)]
pub struct ConnctlTab {
    is_visible: bool,

    items: Option<Vec<Vec<String>>>,
    state: TableState,

    util: SharedBackend,
    msgpopup: ConfirmPopup,

    op: Option<Cop>,
}

impl ConnctlTab {
    pub fn new(util: SharedBackend) -> Self {
        let mut instance = Self {
            is_visible: false,
            items: None,
            state: Default::default(),
            util,
            msgpopup: Default::default(),
            op: None,
        };
        instance.refresh();
        instance
    }
    pub fn refresh(&mut self) {
        let vars = match self.util.get_connections() {
            Ok(v) => v,
            Err(e) => {
                self.popup_list_msg(vec![
                    "Failed to fetch connection info from clash".to_string(),
                    format!("ERR message:{e}"),
                ]);
                return;
            }
        };
        if let Some(var) = vars.0 {
            self.items.replace(var);
        }
    }
    pub fn terminate_conn(&mut self) {
        if let Some(index) = self.state.selected() {
            let id = self
                .items
                .as_ref()
                .unwrap()
                .get(index)
                .unwrap()
                .get(6)
                .unwrap();
            if let Err(e) = self.util.terminate_conn(id) {
                self.popup_list_msg(vec![
                    "Failed to terminate connection".to_string(),
                    format!("ERR message:{e}"),
                ]);
            }
            self.refresh();
        } else {
            unreachable!("Called without a selected item")
        }
    }
    pub fn terminate_all_conns(&mut self) {
        if let Err(e) = self.util.terminate_all_conns() {
            self.popup_list_msg(vec![
                "Failed to terminate all connections".to_string(),
                format!("ERR message:{e}"),
            ]);
        }
        self.refresh();
    }
}

impl super::TabEvent for ConnctlTab {
    fn draw(&mut self, f: &mut ratatui::prelude::Frame, area: ratatui::prelude::Rect) {
        use ratatui::{
            layout::Alignment,
            text::Text,
            widgets::{Block, Borders, Row, Table},
        };
        if !self.is_visible {
            return;
        }
        use ratatui::prelude::Constraint;
        const BAR: [&str; 6] = ["Name", "Url", "Type", "Start Time", "Recvived", "Send"];
        let cur_width = f.size().width;
        let col_styles = [
            Alignment::Left,
            Alignment::Left,
            Alignment::Center,
            Alignment::Left,
            Alignment::Center,
            Alignment::Center,
        ];
        let header = Row::new(BAR.into_iter().map(|s| Text::from(s).centered()));
        let rows: Vec<Row> = self
            .items
            .iter()
            .flat_map(|i| i.iter())
            .map(|l| {
                Row::new(
                    l.iter()
                        .take(6)
                        .zip(col_styles)
                        .map(|(s, a)| Text::from(s.as_str()).alignment(a)),
                )
            })
            .collect();
        let tabs = Table::new(
            rows,
            [
                Constraint::Length(cur_width / 5),
                Constraint::Length(cur_width / 5),
                Constraint::Length(cur_width / 10),
                Constraint::Length(cur_width / 5),
                Constraint::Length(cur_width / 10),
                Constraint::Length(cur_width / 10),
            ],
        )
        .header(header);

        f.render_stateful_widget(
            tabs.block(Block::default().borders(Borders::ALL).title("Connections"))
                .highlight_symbol("█"),
            area,
            &mut self.state,
        );

        self.msgpopup.draw(f, area);
    }

    fn popup_event(&mut self, ev: &ui::event::Event) -> Result<EventState, ui::Infailable> {
        if !self.is_visible {
            return Ok(EventState::NotConsumed);
        }
        let event_state = self.msgpopup.event(ev)?;
        match event_state {
            EventState::Yes => {
                if let Some(op) = self.op.as_ref() {
                    match op {
                        Cop::ShowInfo | Cop::Refresh | Cop::Terminate | Cop::TerminateAll => {
                            unreachable!()
                        }
                        Cop::PreTerminate => {
                            self.op.replace(Cop::Terminate);
                        }
                        Cop::PreTerminateAll => {
                            self.op.replace(Cop::TerminateAll);
                        }
                        Cop::Indialog => {
                            self.op.replace(Cop::PreTerminate);
                            self.msgpopup
                                .popup_confirm("Sure to terminate this connection?".to_owned());
                        }
                    }
                    return Ok(EventState::WorkDone);
                }
                if self.op.as_ref().is_some_and(|p| *p == Cop::PreTerminate) {
                    self.op.replace(Cop::Terminate);
                    Ok(EventState::WorkDone)
                } else {
                    Ok(EventState::NotConsumed)
                }
            }
            EventState::Cancel => {
                self.op.take();
                Ok(EventState::WorkDone)
            }
            _ => Ok(event_state),
        }
    }

    fn event(&mut self, ev: &ui::event::Event) -> Result<EventState, ui::Infailable> {
        if !self.is_visible {
            return Ok(EventState::NotConsumed);
        }

        if let ui::event::Event::Key(key) = ev {
            if key.kind != ui::event::KeyEventKind::Press {
                return Ok(EventState::NotConsumed);
            }
            match key.code.into() {
                Keys::Up => self.previous(),
                Keys::Down => self.next(),
                Keys::ConnRefresh => {
                    self.op.replace(Cop::Refresh);
                }
                Keys::Select => {
                    self.op.replace(Cop::ShowInfo);
                }
                Keys::Search => todo!(),
                _ => return Ok(EventState::NotConsumed),
            }
            return Ok(EventState::WorkDone);
        }
        return Ok(EventState::NotConsumed);
    }

    fn late_event(&mut self) {
        if let Some(op) = self.op.take() {
            match op {
                Cop::Refresh => self.refresh(),
                Cop::PreTerminateAll | Cop::PreTerminate | Cop::Indialog => {
                    self.op.replace(op);
                }
                Cop::ShowInfo => {
                    self.msgpopup.popup_confirm("in dev".to_string());
                    self.op.replace(Cop::Indialog);
                }
                Cop::Terminate => self.terminate_conn(),
                Cop::TerminateAll => self.terminate_all_conns(),
            }
        }
    }
}
impl ConnctlTab {
    pub fn next(&mut self) {
        if let Some(items) = self.items.as_ref() {
            let i = match self.state.selected() {
                Some(i) => {
                    if i >= items.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.state.select(Some(i));
            //self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
        }
    }

    pub fn previous(&mut self) {
        if let Some(items) = self.items.as_ref() {
            let i = match self.state.selected() {
                Some(i) => {
                    if i == 0 {
                        items.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.state.select(Some(i));
            //self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
        }
    }
}
msgpopup_methods!(ConnctlTab);
