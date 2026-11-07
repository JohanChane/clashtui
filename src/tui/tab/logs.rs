use super::dev::*;
use crate::functions::restful::api_log::LogEntry;
use crate::tui::business::{Business, BusinessView};
use ratatui::layout::{Constraint, Layout};
use ratatui::text::Line;
use ratatui::widgets::{List, ListItem, Paragraph, Wrap};
use std::sync::Arc;

newtype_tab!(LogsTab(Tab<Logs>), Logs);

key_map!(
    Logs,
    Action,
    DefaultBindings::new()
        .with_keys([
            (KeyCode::Up, Action::MoveUp),
            (KeyCode::Down, Action::MoveDown),
            (KeyCode::Char('k'), Action::MoveUp),
            (KeyCode::Char('j'), Action::MoveDown),
            (KeyCode::Char('G'), Action::GoBottom),
            (KeyCode::Char('/'), Action::Search),
            (KeyCode::Char('p'), Action::TogglePause),
            (KeyCode::Char('f'), Action::FzfFind),
            (KeyCode::Char('c'), Action::Clear),
        ])
        .with_chords(KeyCode::Char('g'), [(KeyCode::Char('g'), Action::GoTop)])
        .with_chords(
            KeyCode::Char('t'),
            [
                (KeyCode::Char('d'), Action::ToggleDebug),
                (KeyCode::Char('i'), Action::ToggleInfo),
                (KeyCode::Char('w'), Action::ToggleWarning),
                (KeyCode::Char('e'), Action::ToggleError),
            ]
        )
);

#[derive_aliases::derive(..Action, Debug)]
pub(in crate::tui) enum Action {
    MoveUp,
    MoveDown,
    GoTop,
    GoBottom,
    Search,
    TogglePause,
    FzfFind,
    Clear,
    ToggleDebug,
    ToggleInfo,
    ToggleWarning,
    ToggleError,
}

impl Document for Action {
    fn get_doc(&self) -> &'static str {
        use crate::tui::key::consts::*;
        match self {
            Self::MoveUp => MOVE_UP,
            Self::MoveDown => MOVE_DOWN,
            Self::GoTop => GO_TOP,
            Self::GoBottom => GO_BOTTOM,
            Self::Search => FILTER,
            Self::TogglePause => PAUSE,
            Self::FzfFind => "Find",
            Self::Clear => "Clear logs",
            Self::ToggleDebug => "Toggle Debug",
            Self::ToggleInfo => "Toggle Info",
            Self::ToggleWarning => "Toggle Warning",
            Self::ToggleError => "Toggle Error",
        }
    }
}

mod collector;
pub(in crate::tui) use collector::{LogCollector, Snapshot};

struct Logs {
    entries: Vec<Arc<LogEntry>>,
    scroll: usize,
    error: Option<String>,
    filter: Option<String>,
    paused: bool,
    current_log_level: String,
}

impl Default for Logs {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            scroll: 0,
            error: None,
            filter: None,
            paused: true,
            current_log_level: "info".to_owned(),
        }
    }
}

impl Logs {
    fn read_collector(&mut self, view: BusinessView<'_>) {
        let follow_tail =
            self.entries.is_empty() || self.scroll >= self.entries.len().saturating_sub(1);
        let snapshot = view.log_snapshot();
        self.entries = snapshot.entries;
        self.paused = snapshot.paused;
        self.current_log_level = snapshot.level;
        self.error = snapshot.diagnostic;
        if follow_tail {
            self.scroll = self.entries.len().saturating_sub(1);
        } else {
            self.scroll = self.scroll.min(self.entries.len().saturating_sub(1));
        }
    }
}

impl BasicTabContent for Logs {
    type Action = Action;
    type State = ();

    const TITLE: &str = "Logs";
    fn sync_from(&mut self, view: BusinessView<'_>, _: &mut ()) {
        self.read_collector(view);
    }

    fn on_enter(
        &mut self,
        _task_set: &mut FutureSet<Self>,
        _state: &mut Self::State,
        cx: &mut Business,
    ) {
        cx.confirm_api();
        self.read_collector(cx.view());
    }
}

impl TabContent for Logs {
    fn init(&mut self, _task_set: &mut FutureSet<Self>, _state: &mut Self::State) {}

    fn handle_action(
        &mut self,
        action: Action,
        task_set: &mut FutureSet<Self>,
        _state: &mut Self::State,
        cx: &mut Business,
    ) {
        match action {
            Action::MoveUp => self.scroll = self.scroll.saturating_sub(1),
            Action::MoveDown => {
                if self.scroll + 1 < self.entries.len() {
                    self.scroll += 1;
                }
            }
            Action::GoTop => self.scroll = 0,
            Action::GoBottom => self.scroll = self.entries.len().saturating_sub(1),
            Action::Search => {
                async move {
                    let filter = tri!(
                        Input::new()
                            .with_title("Filter".to_owned())
                            .build_and_send_async()
                            .await
                            .await,
                        or_cancel
                    );
                    wrapper(move |content: &mut Logs| {
                        content.filter = (!filter.is_empty()).then_some(filter);
                    })
                }
                .spawn_at(task_set);
            }
            Action::TogglePause => {
                cx.pause_logs(!self.paused);
                self.read_collector(cx.view());
            }
            Action::FzfFind => {
                cx.pause_logs(true);
                self.read_collector(cx.view());
                let names: Vec<String> = self
                    .entries
                    .iter()
                    .map(|entry| format!("{} {} {}", entry.time, entry.type_, entry.payload))
                    .collect();
                async move {
                    let selected = FzfFinder::new(names)
                        .with_title("Find Log")
                        .build_and_send_async()
                        .await
                        .await
                        .unwrap_or_default();
                    wrapper(move |content: &mut Logs| {
                        if let Some(index) = selected {
                            content.scroll = index;
                        }
                    })
                }
                .spawn_at(task_set);
            }
            Action::Clear => {
                cx.clear_logs();
                self.scroll = 0;
                self.filter = None;
                self.read_collector(cx.view());
            }
            Action::ToggleDebug
            | Action::ToggleInfo
            | Action::ToggleWarning
            | Action::ToggleError => {
                if cx.view().core.api_allowed() {
                    let level = match action {
                        Action::ToggleDebug => "debug",
                        Action::ToggleInfo => "info",
                        Action::ToggleWarning => "warning",
                        Action::ToggleError => "error",
                        _ => unreachable!(),
                    };
                    cx.set_log_level(level);
                    self.read_collector(cx.view());
                }
            }
        }
    }

    fn render(&self, f: &mut Frame, area: Rect, _state: &mut Self::State, _view: BusinessView<'_>) {
        let mut block = Block::bordered()
            .border_style(Theme::get().section("logs").border)
            .title(match &self.filter {
                Some(filter) => format!("{} / {filter}", Self::TITLE),
                None => Self::TITLE.to_owned(),
            });
        let mut title_parts = vec![self.current_log_level.clone()];
        if self.paused {
            title_parts.push("[PAUSED]".to_owned());
        }
        block = block.title_bottom(Line::raw(title_parts.join(" ")).right_aligned().reversed());
        let mut log_area = block.inner(area);
        f.render_widget(block, area);
        if log_area.is_empty() {
            return;
        }
        let lines: Vec<ListItem> = self
            .entries
            .iter()
            .map(|entry| format!("{} {} {}", entry.time, entry.type_, entry.payload))
            .filter(|line| {
                self.filter
                    .as_deref()
                    .is_none_or(|pattern| line.contains(pattern))
            })
            .map(|line| ListItem::new(Line::raw(line)))
            .collect();

        if let Some(error) = &self.error {
            // Keep connection status outside the scrollable, filtered history.
            // Short screens retain at least one history row when two rows fit.
            let status_height = if lines.is_empty() {
                log_area.height
            } else {
                log_area.height.div_ceil(2).min(2)
            };
            let [status_area, history_area] =
                Layout::vertical([Constraint::Length(status_height), Constraint::Fill(1)])
                    .areas(log_area);
            f.render_widget(
                Paragraph::new(error.as_str()).wrap(Wrap { trim: false }),
                status_area,
            );
            log_area = history_area;
        }

        let list = List::new(lines).highlight_style(Theme::get().section("logs").highlight);
        let mut state = ratatui::widgets::ListState::default().with_selected(Some(self.scroll));
        f.render_stateful_widget(list, log_area, &mut state);
    }
}
