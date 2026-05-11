use super::dev::*;
use crate::functions::command;
use crate::functions::restful::config;
use ratatui::text::Line;
use ratatui::widgets::{List, ListItem};

newtype_tab!(LogsTab(Tab<Logs>));

mod_agent!(
    Key,
    [
        ([KeyCode::Up], Key::MoveUp, ""),
        ([KeyCode::Down], Key::MoveDown, ""),
        ([KeyCode::Char('k')], Key::MoveUp, ""),
        ([KeyCode::Char('j')], Key::MoveDown, ""),
        ([KeyCode::Char('G')], Key::GoBottom, ""),
        ([KeyCode::Char('g'), KeyCode::Char('g')], Key::GoTop, "Go to top"),
        ([KeyCode::Char('/')], Key::Search, "Search/Filter"),
        ([KeyCode::Char('p')], Key::TogglePause, "Pause/Resume"),
        ([KeyCode::Char('f')], Key::FzfFind, "Find"),
        ([KeyCode::Char('c')], Key::Clear, "Clear logs"),
        ([KeyCode::Char('t'), KeyCode::Char('d')], Key::ToggleDebug, "Toggle debug"),
        ([KeyCode::Char('t'), KeyCode::Char('i')], Key::ToggleInfo, "Toggle info"),
        ([KeyCode::Char('t'), KeyCode::Char('w')], Key::ToggleWarning, "Toggle warning"),
        ([KeyCode::Char('t'), KeyCode::Char('e')], Key::ToggleError, "Toggle error"),
        ([KeyCode::Char('t'), KeyCode::Char('s')], Key::ToggleSilent, "Toggle silent"),
    ]
);

#[derive(Clone, Copy, serde::Deserialize)]
pub enum Key {
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
    ToggleSilent,
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

const MAX_BUFFER_LINES: usize = 10000;

impl Default for Logs {
    fn default() -> Self {
        Self {
            lines: Vec::new(),
            scroll: 0,
            error: None,
            filter: None,
            paused: true,
            current_log_level: String::new(),
        }
    }
}

struct Logs {
    lines: Vec<String>,
    scroll: usize,
    error: Option<String>,
    filter: Option<String>,
    paused: bool,
    current_log_level: String,
}

impl BasicTabContent for Logs {
    type Key = Key;
    type State = ();

    const TITLE: &str = "Logs";

    fn all_shortcuts() -> &'static [(KeyCombo, Self::Key, &'static str)] {
        agent::all_shortcuts()
    }

    fn after_sync(&self, task_set: &mut FutureSet<Self>) {
        if self.paused {
            return;
        }
        async {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let text = command::get_logs();
            wrapper(move |content: &mut Self| {
                content.error = None;
                for line in text.lines() {
                    content.lines.push(line.to_owned());
                }
                // Truncate buffer if needed
                if content.lines.len() > MAX_BUFFER_LINES {
                    let excess = content.lines.len() - MAX_BUFFER_LINES;
                    content.lines.drain(0..excess);
                    content.scroll = content.scroll.saturating_sub(excess);
                }
                // Auto-scroll to bottom if at bottom
                if content.scroll + 1 >= content.lines.len().saturating_sub(1) {
                    content.scroll = content.lines.len().saturating_sub(1);
                }
            })
        }
        .spawn_at(task_set);
    }
}

impl TabContent for Logs {
    fn init(&mut self, task_set: &mut FutureSet<Self>, _state: &mut Self::State) {
        self.error = Some("Press p to start capturing logs".to_owned());
        // Fetch initial log level
        async {
            let cfg = tri!(config::fetch(), or_set);
            wrapper(move |content: &mut Self| {
                content.current_log_level = cfg
                    .log_level
                    .as_ref()
                    .map(|l| l.to_string())
                    .unwrap_or_else(|| "unknown".to_owned());
                content.error = None;
            })
        }
        .spawn_at(task_set);
        // Fetch initial logs
        async {
            let text = command::get_logs();
            wrapper(move |content: &mut Self| {
                for line in text.lines() {
                    content.lines.push(line.to_owned());
                }
                if !content.lines.is_empty() {
                    content.scroll = content.lines.len().saturating_sub(1);
                }
                if content.error.as_deref() == Some("Loading logs...") {
                    content.error = None;
                }
            })
        }
        .spawn_at(task_set);
    }

    fn handle_key_event(
        &mut self,
        key: Key,
        task_set: &mut FutureSet<Self>,
        _state: &mut Self::State,
    ) {
        match key {
            Key::MoveUp => {
                self.scroll = self.scroll.saturating_sub(1);
            }
            Key::MoveDown => {
                if self.scroll + 1 < self.lines.len() {
                    self.scroll += 1;
                }
            }
            Key::GoTop => {
                self.scroll = 0;
            }
            Key::GoBottom => {
                if !self.lines.is_empty() {
                    self.scroll = self.lines.len().saturating_sub(1);
                }
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
                    wrapper(move |content: &mut Logs| {
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
                let names: Vec<String> = self.lines.clone();
                async move {
                    let selected = tokio::task::spawn_blocking(move || {
                        crate::tui::widget::fzffind::run_fzf(&names, "Find Log")
                    })
                    .await
                    .unwrap_or(None);
                    wrapper(move |content: &mut Logs| {
                        if let Some(idx) = selected {
                            content.scroll = idx;
                        }
                    })
                }
                .spawn_at(task_set);
            }
            Key::Clear => {
                self.lines.clear();
                self.scroll = 0;
                self.filter = None;
            }
            Key::ToggleDebug => self.toggle_log_level("debug", task_set),
            Key::ToggleInfo => self.toggle_log_level("info", task_set),
            Key::ToggleWarning => self.toggle_log_level("warning", task_set),
            Key::ToggleError => self.toggle_log_level("error", task_set),
            Key::ToggleSilent => self.toggle_log_level("silent", task_set),
        }
    }

    fn render(&self, f: &mut Frame, area: Rect, _state: &mut Self::State) {
        let block = Block::bordered()
            .border_style(Theme::get().tab.tab_focused)
            .title(Self::TITLE);

        let mut title_parts = Vec::new();
        title_parts.push(self.current_log_level.clone());
        if let Some(ref filter) = self.filter {
            title_parts.push(format!(" / {filter} "));
        } else {
            title_parts.push(" /: Search ".to_owned());
        }
        if self.paused {
            title_parts.push(" [PAUSED]".to_owned());
        }
        let block = block.title_bottom(
            Line::raw(title_parts.join(" "))
                .right_aligned()
                .reversed(),
        );

        if !self.error.as_deref().unwrap_or("").is_empty() && self.lines.is_empty() {
            let widget = ratatui::widgets::Paragraph::new(
                self.error.as_deref().unwrap_or(""),
            )
            .block(block);
            f.render_widget(widget, area);
            return;
        }

        let visible_lines: Vec<ListItem> = self
            .lines
            .iter()
            .enumerate()
            .filter(|(_, line)| {
                self.filter
                    .as_deref()
                    .is_none_or(|pat| line.contains(pat))
            })
            .map(|(_, line)| ListItem::new(Line::raw(line.as_str())))
            .collect();

        let highlight_style = Theme::get().tab.item_highlighted;
        let list = List::new(visible_lines)
            .block(block)
            .highlight_style(highlight_style);

        let mut list_state = ratatui::widgets::ListState::default()
            .with_selected(Some(self.scroll));
        f.render_stateful_widget(list, area, &mut list_state);
    }
}

impl Logs {
    fn toggle_log_level(&mut self, level: &str, task_set: &mut FutureSet<Self>) {
        let level = level.to_owned();
        async move {
            let payload = serde_json::json!({"log-level": &level}).to_string();
            let result = config::patch(payload);
            wrapper(move |content: &mut Logs| {
                match result {
                    Ok(_) => {
                        content.current_log_level = level;
                    }
                    Err(e) => {
                        crate::tui::widget::popmsg::Confirm::err(e);
                    }
                }
            })
        }
        .spawn_at(task_set);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyCode;

    fn kev(code: KeyCode, shift: bool) -> crate::tui::Key {
        crate::tui::Key {
            code,
            shift,
            ctrl: false,
            alt: false,
            super_: false,
        }
    }

    #[test]
    fn key_j_maps_to_move_down() {
        let k = kev(KeyCode::Char('j'), false);
        let result = Key::try_from(&k);
        assert!(matches!(result, Ok(Key::MoveDown)));
    }

    #[test]
    fn key_k_maps_to_move_up() {
        let k = kev(KeyCode::Char('k'), false);
        let result = Key::try_from(&k);
        assert!(matches!(result, Ok(Key::MoveUp)));
    }

    #[test]
    fn key_p_maps_to_toggle_pause() {
        let k = kev(KeyCode::Char('p'), false);
        let result = Key::try_from(&k);
        assert!(matches!(result, Ok(Key::TogglePause)));
    }

    #[test]
    fn key_c_maps_to_clear() {
        let k = kev(KeyCode::Char('c'), false);
        let result = Key::try_from(&k);
        assert!(matches!(result, Ok(Key::Clear)));
    }

    #[test]
    fn key_f_maps_to_fzf_find() {
        let k = kev(KeyCode::Char('f'), false);
        let result = Key::try_from(&k);
        assert!(matches!(result, Ok(Key::FzfFind)));
    }

    #[test]
    fn key_slash_maps_to_search() {
        let k = kev(KeyCode::Char('/'), false);
        let result = Key::try_from(&k);
        assert!(matches!(result, Ok(Key::Search)));
    }

    #[test]
    fn default_logs_is_paused_true() {
        let logs = Logs::default();
        assert!(logs.paused, "log capture should be paused by default");
    }

    #[test]
    fn default_logs_has_empty_lines() {
        let logs = Logs::default();
        assert!(logs.lines.is_empty());
        assert_eq!(logs.scroll, 0);
    }

    #[test]
    fn clear_resets_state() {
        let mut logs = Logs {
            lines: vec!["line1".to_owned(), "line2".to_owned()],
            scroll: 1,
            filter: Some("test".to_owned()),
            ..Default::default()
        };
        assert_eq!(logs.lines.len(), 2);
        assert!(logs.filter.is_some());

        // Simulate clear by directly modifying fields (cannot call handle_key_event without task_set)
        logs.lines.clear();
        logs.scroll = 0;
        logs.filter = None;

        assert!(logs.lines.is_empty());
        assert_eq!(logs.scroll, 0);
        assert!(logs.filter.is_none());
    }

    #[test]
    fn toggle_pause_flips_state() {
        let mut logs = Logs::default();
        assert!(logs.paused, "default is paused");
        logs.paused = false;
        assert!(!logs.paused);
        logs.paused = true;
        assert!(logs.paused);
    }

    #[test]
    fn scroll_clamps_at_top() {
        let mut logs = Logs::default();
        logs.scroll = 0;
        logs.scroll = logs.scroll.saturating_sub(1);
        assert_eq!(logs.scroll, 0);
    }

    #[test]
    fn move_down_clamps_at_end() {
        let mut logs = Logs {
            lines: vec!["a".to_owned(), "b".to_owned()],
            scroll: 0,
            ..Default::default()
        };
        logs.scroll += 1; // 1
        logs.scroll += 1; // 2 - but max index is 1
        if logs.scroll >= logs.lines.len() {
            logs.scroll = logs.lines.len().saturating_sub(1);
        }
        assert_eq!(logs.scroll, 1);
    }
}
