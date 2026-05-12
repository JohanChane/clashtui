use super::dev::*;
use crate::functions::restful::connection::{self, Conn};
use crate::tui::widget::fzffind;
use ratatui::text::Line;
use ratatui::widgets::{Cell, Row, Table};
use std::collections::HashMap;

newtype_tab!(ConnectionsTab(Tab<Connections>));

mod_agent!(
    Key,
    [
        ([KeyCode::Up], Key::MoveUp, ""),
        ([KeyCode::Down], Key::MoveDown, ""),
        ([KeyCode::Char('k')], Key::MoveUp, ""),
        ([KeyCode::Char('j')], Key::MoveDown, ""),
        ([KeyCode::Char('G')], Key::GoBottom, ""),
        ([KeyCode::Char('g'), KeyCode::Char('g')], Key::GoTop, "Go to top"),
        ([KeyCode::Char('d'), KeyCode::Char('d')], Key::Terminate, "Close"),
        ([KeyCode::Char('a'), KeyCode::Char('c')], Key::TerminateAll, "Close all"),
        ([KeyCode::Char('s'), KeyCode::Char('d')], Key::SortByDownload, "Sort by DL speed"),
        ([KeyCode::Char('s'), KeyCode::Char('u')], Key::SortByUpload, "Sort by UL speed"),
        ([KeyCode::Char('s'), KeyCode::Char('r')], Key::SortReset, "Reset sort"),
        ([KeyCode::Char('/')], Key::Search, "Search/Filter"),
        ([KeyCode::Char('p')], Key::TogglePause, "Pause/Resume"),
        ([KeyCode::Char('f')], Key::FzfFind, "Find"),
    ]
);

#[derive(Clone, Copy, serde::Deserialize)]
pub enum Key {
    MoveUp,
    MoveDown,
    GoTop,
    GoBottom,
    Terminate,
    TerminateAll,
    SortByDownload,
    SortByUpload,
    SortReset,
    Search,
    TogglePause,
    FzfFind,
}

impl TryFrom<&crate::tui::Key> for Key {
    type Error = ();

    fn try_from(ev: &crate::tui::Key) -> Result<Self, Self::Error> {
        let agent = agent();
        if !agent.is_empty() {
            return agent.get(ev).map(|act| *act).ok_or(());
        }
        Err(())
    }
}

macro_rules! tri {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => {
                crate::tui::widget::popmsg::Confirm::err(e);
                return do_nothing();
            }
        }
    };
    ($e:expr, or_cancel) => {
        match $e {
            Ok(v) => v,
            Err(_) => return do_nothing(),
        }
    };
    ($e:expr, or_set) => {
        match $e {
            Ok(v) => v,
            Err(e) => {
                return wrapper(move |content: &mut Self| {
                    content.error = Some(e.to_string());
                });
            }
        }
    };
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum SortState {
    #[default]
    Default,
    ByDownload,
    ByUpload,
}

struct DisplayRow {
    host: String,
    rule: String,
    chains: String,
    download: u64,
    upload: u64,
    dl_speed: u64,
    ul_speed: u64,
    id: String,
}

#[derive(Default)]
struct Connections {
    conns: Vec<Conn>,
    display_rows: Vec<DisplayRow>,
    row: Option<usize>,
    error: Option<String>,
    last_bytes: HashMap<String, (u64, u64)>,
    sort_state: SortState,
    filter: Option<String>,
    paused: bool,
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{size:.0} {unit}", size = size, unit = UNITS[unit_idx])
    } else {
        format!("{size:.1} {unit}", size = size, unit = UNITS[unit_idx])
    }
}

fn human_speed(bytes_per_sec: u64) -> String {
    format!("{}/s", human_bytes(bytes_per_sec))
}

fn make_display_rows(conns: &[Conn], last_bytes: &mut HashMap<String, (u64, u64)>) -> Vec<DisplayRow> {
    conns
        .iter()
        .map(|c| {
            let host = c.metadata.host.clone();
            let port = &c.metadata.destination_port;
            let host_display = if host.is_empty() {
                if let Some(ref ip) = c.metadata.destination_ip {
                    if !port.is_empty() && port != "0" {
                        format!("{ip}:{port}")
                    } else {
                        ip.clone()
                    }
                } else {
                    c.metadata.remote_destination.clone()
                }
            } else {
                if !port.is_empty() && port != "0" {
                    format!("{host}:{port}")
                } else {
                    host
                }
            };

            let rule = c.rule.as_deref().unwrap_or("-");
            let chains = {
                let mut rev: Vec<&str> = c.chains.iter().map(|s| s.as_str()).collect();
                rev.reverse();
                if rev.is_empty() {
                    "-".to_owned()
                } else {
                    rev.join(" > ")
                }
            };

            let prev = last_bytes.get(&c.id).copied().unwrap_or((c.download, c.upload));
            let dl_speed = c.download.saturating_sub(prev.0);
            let ul_speed = c.upload.saturating_sub(prev.1);

            last_bytes.insert(c.id.clone(), (c.download, c.upload));

            DisplayRow {
                host: host_display,
                rule: rule.to_owned(),
                chains,
                download: c.download,
                upload: c.upload,
                dl_speed,
                ul_speed,
                id: c.id.clone(),
            }
        })
        .collect()
}

const HOST_COL: &str = "Host";
const RULE_COL: &str = "Rule";
const CHAINS_COL: &str = "Chains";
const DL_COL: &str = "Download";
const UL_COL: &str = "Upload";
const DLSPD_COL: &str = "DL Speed";
const ULSPD_COL: &str = "UL Speed";

impl BasicTabContent for Connections {
    type Key = Key;
    type State = ();

    const TITLE: &str = "Connections";

    fn all_shortcuts() -> &'static [(KeyCombo, Self::Key, &'static str)] {
        agent::all_shortcuts()
    }

    fn after_sync(&self, task_set: &mut FutureSet<Self>) {
        if self.paused {
            return;
        }
        if crate::config::is_core_mismatch() {
            return;
        }
        async {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let info = tri!(connection::get_connections(), or_set);
            wrapper(|content: &mut Self| {
                let conns = info.connections.unwrap_or_default();
                content.conns = conns;
                content.error = None;
                content.refresh_display_rows();
            })
        }
        .spawn_at(task_set);
    }

    fn on_enter(&mut self, task_set: &mut FutureSet<Self>, _state: &mut Self::State) {
        self.paused = false;
        if crate::config::is_core_mismatch() {
            self.conns = Vec::new();
            self.display_rows = Vec::new();
            self.error = Some("API data mismatch with configured core".to_owned());
            return;
        }
        async {
            let info = tri!(connection::get_connections(), or_set);
            wrapper(|content: &mut Self| {
                let conns = info.connections.unwrap_or_default();
                content.conns = conns;
                content.error = None;
                content.refresh_display_rows();
            })
        }
        .spawn_at(task_set);
    }

    fn on_leave(&mut self, _task_set: &mut FutureSet<Self>, _state: &mut Self::State) {
        self.paused = true;
    }
}

impl TabContent for Connections {
    fn init(&mut self, _task_set: &mut FutureSet<Self>, _state: &mut Self::State) {
        self.paused = true;
        self.error = Some("Loading connections...".to_owned());
    }

    fn handle_key_event(
        &mut self,
        key: Key,
        task_set: &mut FutureSet<Self>,
        _state: &mut Self::State,
    ) {
        match key {
            Key::MoveUp => {
                if let Some(r) = self.row {
                    if r > 0 {
                        self.row = Some(r - 1);
                    }
                } else if !self.display_rows.is_empty() {
                    self.row = Some(self.display_rows.len() - 1);
                }
            }
            Key::MoveDown => {
                if let Some(r) = self.row {
                    if r + 1 < self.display_rows.len() {
                        self.row = Some(r + 1);
                    }
                } else if !self.display_rows.is_empty() {
                    self.row = Some(0);
                }
            }
            Key::GoTop => {
                if !self.display_rows.is_empty() {
                    self.row = Some(0);
                }
            }
            Key::GoBottom => {
                if !self.display_rows.is_empty() {
                    self.row = Some(self.display_rows.len().saturating_sub(1));
                }
            }
            Key::Terminate => {
                let Some(row) = self.row else { return };
                let Some(display_row) = self.display_rows.get(row) else { return };
                let id = display_row.id.clone();
                async move {
                    let _ = connection::terminate_connection(Some(id));
                    let info = tri!(connection::get_connections(), or_cancel);
                    wrapper(move |content: &mut Connections| {
                        content.conns = info.connections.unwrap_or_default();
                        content.error = None;
                        content.refresh_display_rows();
                        if content.row.unwrap_or(0) >= content.display_rows.len() {
                            content.row = content.display_rows.len().checked_sub(1);
                        }
                    })
                }
                .spawn_at(task_set);
            }
            Key::TerminateAll => {
                let (use_bulk, ids): (bool, Vec<String>) =
                    if let Some(ref pat) = self.filter {
                        let ids: Vec<String> = self
                            .display_rows
                            .iter()
                            .filter(|r| {
                                r.host.contains(pat)
                                    || r.rule.contains(pat)
                                    || r.chains.contains(pat)
                                    || r.id.contains(pat)
                            })
                            .map(|r| r.id.clone())
                            .collect();
                        (false, ids)
                    } else {
                        (true, Vec::new())
                    };

                let count = if self.filter.is_some() {
                    ids.len()
                } else {
                    self.display_rows.len()
                };

                if count == 0 {
                    return;
                }

                async move {
                    if use_bulk {
                        let _ = connection::terminate_all_connections();
                    } else {
                        for id in &ids {
                            let _ = connection::terminate_connection(Some(id.clone()));
                        }
                    }
                    let info = tri!(connection::get_connections(), or_cancel);
                    wrapper(move |content: &mut Connections| {
                        content.conns = info.connections.unwrap_or_default();
                        content.error = None;
                        content.refresh_display_rows();
                        content.row = None;
                    })
                }
                .spawn_at(task_set);
            }
            Key::SortByDownload => {
                self.sort_state = SortState::ByDownload;
                self.apply_sort();
            }
            Key::SortByUpload => {
                self.sort_state = SortState::ByUpload;
                self.apply_sort();
            }
            Key::SortReset => {
                self.sort_state = SortState::Default;
                self.apply_sort();
            }
            Key::Search => {
                async move {
                    let filter = tri!(
                        Input::new()
                            .with_title("Filter".to_owned())
                            .build_and_send()
                            .await,
                        or_cancel
                    );
                    wrapper(move |content: &mut Connections| {
                        content.filter = (!filter.is_empty()).then_some(filter);
                    })
                }
                .spawn_at(task_set);
            }
            Key::TogglePause => {
                self.paused = !self.paused;
            }
            Key::FzfFind => {
                self.paused = true;
                let names: Vec<String> = self.display_rows.iter()
                    .map(|r| format!("{} | {} | {}", r.host, r.rule, r.chains))
                    .collect();
                async move {
                    let selected = tokio::task::spawn_blocking(move || {
                        fzffind::run_fzf(&names, "Find Connection")
                    })
                    .await
                    .unwrap_or(None);
                    wrapper(move |content: &mut Connections| {
                        content.row = selected;
                    })
                }
                .spawn_at(task_set);
            }
        }
    }

    fn render(&self, f: &mut Frame, area: Rect, _state: &mut Self::State) {
        let block = Block::bordered()
            .border_style(Theme::get().tab.tab_focused)
            .title(Self::TITLE);

        let mut title = if let Some(filter) = self.filter.as_ref() {
            format!(" / {filter} ")
        } else {
            " /: Search ".to_owned()
        };
        if self.paused {
            title.push_str(" [PAUSED]");
        }
        let block = block.title_bottom(Line::raw(title).right_aligned().reversed());

        if !self.error.as_deref().unwrap_or("").is_empty() && self.display_rows.is_empty() {
            let widget = ratatui::widgets::Paragraph::new(
                self.error.as_deref().unwrap_or("")
            ).block(block);
            f.render_widget(widget, area);
            return;
        }

        let sort_indicator = match self.sort_state {
            SortState::ByDownload => " (DL ▼)",
            SortState::ByUpload => " (UL ▼)",
            SortState::Default => "",
        };

        let filtered_count: usize = self.display_rows.iter()
            .filter(|r| self.filter.as_deref().is_none_or(|pat| {
                r.host.contains(pat) || r.rule.contains(pat) || r.chains.contains(pat) || r.id.contains(pat)
            }))
            .count();

        let count_text = if self.filter.is_some() {
            format!(
                "{}/{} conns{}",
                filtered_count,
                self.display_rows.len(),
                sort_indicator
            )
        } else {
            format!(
                "{} conns{}",
                self.display_rows.len(),
                sort_indicator
            )
        };

        let dl_speed_header = if self.sort_state == SortState::ByDownload {
            "DL Speed ▼"
        } else {
            DLSPD_COL
        };
        let ul_speed_header = if self.sort_state == SortState::ByUpload {
            "UL Speed ▼"
        } else {
            ULSPD_COL
        };

        let header_style = Theme::get().tab.tab_focused;
        let header_cells = [
            HOST_COL,
            RULE_COL,
            CHAINS_COL,
            DL_COL,
            UL_COL,
            dl_speed_header,
            ul_speed_header,
        ]
        .into_iter()
        .map(|h| Cell::from(h).style(header_style));

        let header = Row::new(header_cells).height(1);

        let widths = [
            ratatui::prelude::Constraint::Min(30),
            ratatui::prelude::Constraint::Max(15),
            ratatui::prelude::Constraint::Min(15),
            ratatui::prelude::Constraint::Max(10),
            ratatui::prelude::Constraint::Max(10),
            ratatui::prelude::Constraint::Max(10),
            ratatui::prelude::Constraint::Max(10),
        ];

        let rows: Vec<Row> = self
            .display_rows
            .iter()
            .filter(|r| {
                self.filter.as_deref().is_none_or(|pat| {
                    r.host.contains(pat)
                        || r.rule.contains(pat)
                        || r.chains.contains(pat)
                        || r.id.contains(pat)
                })
            })
            .map(|r| {
                Row::new(vec![
                    Cell::from(r.host.as_str()),
                    Cell::from(r.rule.as_str()),
                    Cell::from(r.chains.as_str()),
                    Cell::from(human_bytes(r.download)),
                    Cell::from(human_bytes(r.upload)),
                    Cell::from(human_speed(r.dl_speed)),
                    Cell::from(human_speed(r.ul_speed)),
                ])
                .height(1)
            })
            .collect();

        let highlight_style = Theme::get().tab.item_highlighted;
        let table = Table::new(rows, widths)
            .header(header)
            .block(block.title_bottom(Line::raw(count_text).right_aligned()))
            .row_highlight_style(highlight_style);

        if let Some(selected) = self.row {
            f.render_stateful_widget(
                table,
                area,
                &mut ratatui::widgets::TableState::new()
                    .with_selected(Some(selected))
                    .with_offset(0),
            );
        } else {
            f.render_stateful_widget(
                table,
                area,
                &mut ratatui::widgets::TableState::default()
                    .with_offset(0),
            );
        }
    }
}

impl Connections {
    fn refresh_display_rows(&mut self) {
        self.display_rows = make_display_rows(&self.conns, &mut self.last_bytes);
        // Store original order index in a separate field would be ideal,
        // but we can rebuild from conns on SortReset since conns retains API order
        self.apply_sort();
        // Clamp cursor to valid range
        if self.display_rows.is_empty() {
            self.row = None;
        } else if let Some(r) = self.row {
            if r >= self.display_rows.len() {
                self.row = Some(self.display_rows.len().saturating_sub(1));
            }
        } else {
            self.row = Some(0);
        }
    }

    fn apply_sort(&mut self) {
        match self.sort_state {
            SortState::ByDownload => {
                self.display_rows
                    .sort_by(|a, b| b.dl_speed.cmp(&a.dl_speed));
            }
            SortState::ByUpload => {
                self.display_rows
                    .sort_by(|a, b| b.ul_speed.cmp(&a.ul_speed));
            }
            SortState::Default => {
                let orig_ids: Vec<String> = self.conns.iter().map(|c| c.id.clone()).collect();
                self.display_rows.sort_by_key(|r| {
                    orig_ids.iter().position(|id| *id == r.id).unwrap_or(usize::MAX)
                });
            }
        }
    }
}
