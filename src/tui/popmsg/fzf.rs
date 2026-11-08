use super::dev::*;
use crate::tui::{theme::Theme, widget::popmsg::PopupLayout};
use ratatui::{
    layout::{Constraint, Layout, Margin},
    text::Line,
    widgets::{Borders, List, ListDirection, ListItem, ListState, Paragraph},
};

pub struct FzfFinder {
    items: Vec<String>,
    input: super::input::Input,
    idx: simsearch::Index<usize>,
    matches: Vec<usize>,
    state: ListState,
}

impl FzfFinder {
    pub fn new(items: Vec<String>) -> Self {
        let cfg = simsearch::Options::new().separators(['_', ':', '.']);
        let mut idx = simsearch::Index::with_options(cfg);
        for (id, content) in items.iter().enumerate() {
            idx.insert(id, content);
        }

        Self {
            matches: (0..items.len()).collect(),
            state: ListState::default().with_selected((!items.is_empty()).then_some(0)),
            items,
            idx,
            input: Default::default(),
        }
    }

    pub fn with_title(self, title: impl Into<String>) -> MsgBuilder<Self> {
        MsgBuilder::new(self, title.into())
    }

    fn selected(&self) -> Option<usize> {
        self.state
            .selected()
            .and_then(|selected| self.matches.get(selected).copied())
    }

    fn search(&mut self) {
        self.matches = if self.input.buffer.is_empty() {
            (0..self.items.len()).collect()
        } else {
            self.idx
                .search(&self.input.buffer)
                .into_iter()
                .map(|hit| hit.id)
                .collect()
        };
        self.state = ListState::default().with_selected((!self.matches.is_empty()).then_some(0));
    }
}

impl Msg for FzfFinder {
    type Result = Option<usize>;

    fn match_key_event(&mut self, kv: &Key) -> Route {
        match kv.code {
            KeyCode::Up | KeyCode::Down => {
                if let Some(selected) = self.state.selected() {
                    // The first match is at the bottom, next to the input.
                    let selected = if kv.code == KeyCode::Up {
                        selected.saturating_add(1).min(self.matches.len() - 1)
                    } else {
                        selected.saturating_sub(1)
                    };
                    self.state.select(Some(selected));
                }
                Route::Keep
            }
            KeyCode::Enter => {
                if self.selected().is_some() {
                    Route::Send
                } else {
                    Route::Keep
                }
            }
            _ => {
                let previous = self.input.buffer.clone();
                let route = self.input.match_key_event(kv);
                if self.input.buffer != previous {
                    self.search();
                }
                route
            }
        }
    }

    fn send(self, tx: Sender<Self::Result>) {
        let _ = tx.send(self.selected());
    }

    fn layout(&self) -> PopupLayout {
        PopupLayout::Fullscreen
    }

    fn render(&mut self, f: &mut Frame, area: Rect, block: Block, is_focused: bool) {
        let theme = Theme::get();
        let palette = &theme.finder;
        f.render_widget(Block::default().style(palette.text), area);
        let area = area.inner(Margin::new(1, 0));
        let [results, footer] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(2)]).areas(area);

        if self.matches.is_empty() {
            let message = if self.items.is_empty() {
                "No items"
            } else {
                "No matches"
            };
            let empty = Rect {
                y: results.bottom().saturating_sub(1).max(results.y),
                height: results.height.min(1),
                ..results
            };
            f.render_widget(Paragraph::new(message).style(palette.muted), empty);
        } else {
            let items = self
                .matches
                .iter()
                .map(|&id| ListItem::new(Line::raw(self.items[id].as_str())));
            let list = List::new(items)
                .direction(ListDirection::BottomToTop)
                .highlight_symbol("> ")
                .style(palette.text)
                .highlight_style(palette.highlight);
            f.render_stateful_widget(list, results, &mut self.state);
        }

        let footer_block = block
            .style(palette.text)
            .border_style(palette.border)
            .title_style(palette.muted)
            .borders(Borders::TOP)
            .title_top(
                Line::from(format!(" {}/{} ", self.matches.len(), self.items.len()))
                    .right_aligned(),
            );
        self.input.render(f, footer, footer_block, is_focused);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::oneshot;

    fn finder(items: &[&str]) -> FzfFinder {
        FzfFinder::new(items.iter().map(|item| (*item).to_owned()).collect())
    }

    fn press(finder: &mut FzfFinder, code: KeyCode) -> Route {
        finder.match_key_event(&code.into())
    }

    fn confirm(mut finder: FzfFinder) -> Option<usize> {
        assert!(matches!(press(&mut finder, KeyCode::Enter), Route::Send));
        let (tx, mut rx) = oneshot::channel();
        finder.send(tx);
        rx.try_recv().unwrap()
    }

    #[test]
    fn navigation_confirms_the_highlighted_item_without_a_query() {
        let mut finder = finder(&["Paris", "Singapore", "Tokyo"]);
        press(&mut finder, KeyCode::Down);
        assert_eq!(finder.selected(), Some(0));
        for _ in 0..4 {
            press(&mut finder, KeyCode::Up);
        }
        assert_eq!(finder.selected(), Some(2));
        press(&mut finder, KeyCode::Down);
        assert_eq!(confirm(finder), Some(1));
    }

    #[test]
    fn filtering_resets_selection_and_returns_the_original_index() {
        let mut finder = finder(&["Paris", "Singapore", "Tokyo", "Singapore"]);
        press(&mut finder, KeyCode::Up);
        for ch in "Singapore".chars() {
            press(&mut finder, KeyCode::Char(ch));
        }
        assert_eq!(finder.selected(), Some(1));
        press(&mut finder, KeyCode::Up);
        // Moving the text cursor must not reset the selected result.
        press(&mut finder, KeyCode::Left);
        assert_eq!(confirm(finder), Some(3));
    }

    #[test]
    fn log_ports_and_domain_tokens_match_the_original_items() {
        let log = "26-11-08 17:25:50 info [TCP] 198.18.0.1:50180 --> api2.cursor.sh:443 match GeoSite(geolocation-!cn) using Entry[ba]";
        for query in ["443", "cursor", "CURSOR", "api2.cursor.sh:443"] {
            let mut finder = finder(&["unrelated", log, log]);
            for ch in query.chars() {
                press(&mut finder, KeyCode::Char(ch));
            }
            assert_eq!(finder.matches, vec![1, 2], "query: {query}");
            press(&mut finder, KeyCode::Up);
            assert_eq!(confirm(finder), Some(2));
        }
    }

    #[test]
    fn hidden_indices_do_not_match_and_clearing_restores_all_items() {
        let mut finder = finder(&["Paris", "Singapore"]);
        press(&mut finder, KeyCode::Char('0'));
        assert!(finder.matches.is_empty());
        press(&mut finder, KeyCode::Up);
        assert!(matches!(press(&mut finder, KeyCode::Enter), Route::Keep));
        press(&mut finder, KeyCode::Backspace);
        assert_eq!(confirm(finder), Some(0));
    }

    #[test]
    fn empty_finder_cannot_confirm_but_can_cancel() {
        let mut finder = finder(&[]);
        press(&mut finder, KeyCode::Up);
        press(&mut finder, KeyCode::Down);
        assert!(matches!(press(&mut finder, KeyCode::Enter), Route::Keep));
        assert!(matches!(press(&mut finder, KeyCode::Esc), Route::Drop));
    }
}
