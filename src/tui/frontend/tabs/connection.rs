use super::{Call, CallBack, PopMsg, TabCont};
use crate::tui::{
    frontend::consts::TAB_TITLE_CONNECTION,
    widget::{tools, PopRes},
    Drawable, EventState, Theme,
};

use crate::clash::webapi::{Conn, ConnInfo, ConnMetaData};
use ratatui::prelude as Ra;
use ratatui::widgets as Raw;

mod conn;
use conn::Connection;

#[derive(Debug)]
pub enum BackendOp {
    Terminal(String),
    TerminalAll,
}

#[derive(Default)]
pub(in crate::tui::frontend) struct ConnectionTab {
    items: Vec<Connection>,
    filter: Option<String>,
    travel_up: u64,
    travel_down: u64,
    state: Raw::TableState,
    scrollbar: Raw::ScrollbarState,
    selected_con: Option<Box<Connection>>,
    popup_content: Option<PopMsg>,
    backend_content: Option<Call>,
}

impl std::fmt::Display for ConnectionTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", crate::tui::frontend::consts::TAB_TITLE_CONNECTION)
    }
}

impl Drawable for ConnectionTab {
    fn render(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect, _: bool) {
        use Ra::Constraint;
        use Raw::{Block, Borders, Table};
        let tabs = Table::new(
            self.items.iter().map(|l| l.build_col()),
            [
                Constraint::Percentage(25),
                Constraint::Min(6),
                Constraint::Percentage(5),
                Constraint::Max(24),
                Constraint::Max(10),
                Constraint::Max(10),
            ],
        )
        .header(Connection::build_header().style(Theme::get().connection_tab.table_static))
        .footer(
            Connection::build_footer(self.travel_up, self.travel_down)
                .style(Theme::get().connection_tab.table_static),
        )
        .style(if self.selected_con.is_none() {
            Theme::get().list.block_selected
        } else {
            Theme::get().list.block_unselected
        });

        f.render_stateful_widget(
            tabs.block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(TAB_TITLE_CONNECTION),
            )
            .row_highlight_style(Theme::get().list.highlight),
            area,
            &mut self.state,
        );
        if let Some(con) = self.selected_con.as_ref() {
            use Ra::Widget;
            con.render(area, f.buffer_mut());
        }
    }

    /// - Caught event -> [EventState::WorkDone]
    /// - unrecognized event -> [EventState::NotConsumed]
    fn handle_key_event(&mut self, ev: &crossterm::event::KeyEvent) -> EventState {
        use crossterm::event::KeyCode;
        if ev.kind != crossterm::event::KeyEventKind::Press {
            return EventState::NotConsumed;
        }
        // doing popup
        if self.selected_con.is_some() {
            match ev.code {
                KeyCode::Enter => {
                    self.backend_content = self
                        .selected_con
                        .take()
                        .unwrap()
                        .id
                        .map(|id| Call::Connection(BackendOp::Terminal(id)))
                }
                KeyCode::Esc => self.selected_con = None,
                _ => {}
            }
            return EventState::WorkDone;
        }
        match ev.code {
            KeyCode::Enter => {
                if let Some(con) = self
                    .selected()
                    .and_then(|index| self.items.get(index))
                    .map(|c| Box::new(c.clone()))
                {
                    self.selected_con = Some(con);
                }
            }
            KeyCode::Up | KeyCode::Char('k') => self.previous(),
            KeyCode::Down | KeyCode::Char('j') => self.next(),
            KeyCode::Home => {
                self.state.select_first();
                self.scrollbar.first();
            }
            KeyCode::End => {
                self.state.select_last();
                self.scrollbar.last();
            }
            // KeyCode::PageUp => todo!(),
            // KeyCode::PageDown => todo!(),
            _ if crate::tui::frontend::key_bind::Keys::Search == ev.code.into() => {
                self.popup_content = Some(PopMsg::Input("Url/Chain/Type".to_owned()));
            }
            _ if crate::tui::frontend::key_bind::Keys::ConnKillAll == ev.code.into() => {
                self.popup_content = Some(PopMsg::AskChoices(
                    "Are you sure to terminate all connections?\nThis cannot be undone!".to_owned(),
                    vec!["No".to_owned(), "Yes".to_owned()],
                ));
            }
            _ => return EventState::NotConsumed,
        }
        EventState::WorkDone
    }
}

impl TabCont for ConnectionTab {
    fn get_backend_call(&mut self) -> Option<Call> {
        self.backend_content.take()
    }

    fn get_popup_content(&mut self) -> Option<PopMsg> {
        self.popup_content.take()
    }

    fn apply_backend_call(&mut self, op: CallBack) {
        match op {
            CallBack::ConnectionCTL(res) => {
                self.popup_content = Some(PopMsg::Prompt(format!("Done\n{}", res)))
            }
            CallBack::ConnectionInit(items) => {
                let ConnInfo {
                    download_total,
                    upload_total,
                    connections,
                } = items;
                self.items = connections
                    .unwrap_or_default()
                    .into_iter()
                    .map(|c| c.into())
                    .filter(|c: &Connection| {
                        self.filter.as_ref().is_none_or(|pat| c.match_keyword(pat))
                    })
                    .collect();
                self.travel_up = upload_total;
                self.travel_down = download_total;
                // try update track here
                if let Some(con) = self.selected_con.as_mut() {
                    // cannot match this id in list
                    if !self.items.iter().any(|c| c.id == con.id) {
                        con.lose_track();
                    };
                }
            }
            _ => unreachable!("{} get unexpected op: {:?}", TAB_TITLE_CONNECTION, op),
        }
    }

    fn apply_popup_result(&mut self, res: PopRes) -> EventState {
        match res {
            // if we should terminal all connections
            PopRes::Selected(selected) => match selected {
                // regarded as cancel
                0 => (),
                // regarded as yes
                1 => self.backend_content = Some(Call::Connection(BackendOp::TerminalAll)),
                // regarded as extra-choices
                _ => unreachable!(),
            },
            // get filter content
            PopRes::Input(name) => self.filter = Some(name),
            PopRes::SelectedMulti(_) => unreachable!(),
        }
        EventState::WorkDone
    }
}

impl ConnectionTab {
    /// Index of the selected item
    ///
    /// Returns `None` if no item is selected
    pub fn selected(&self) -> Option<usize> {
        self.state.selected()
    }

    fn next(&mut self) {
        self.scrollbar.next();
        self.state.select_next();
    }

    fn previous(&mut self) {
        if self.state.selected().is_none() {
            self.scrollbar.last();
        } else {
            self.scrollbar.prev();
        }
        self.state.select_previous();
    }
}
