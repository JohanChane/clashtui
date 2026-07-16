use super::dev::*;
use ratatui::{text::Text, widgets::Paragraph};

pub struct FzfFinder {
    items: Vec<String>,
    input: super::input::Input,
    idx: simsearch::Index<usize>,
    result_cache: Vec<usize>,
}

impl FzfFinder {
    pub fn new(items: Vec<String>) -> Self {
        let cfg = simsearch::Options::new();
        let mut idx = simsearch::Index::with_options(cfg);
        items
            .iter()
            .enumerate()
            .for_each(|(id, content)| idx.insert(id, &content));
        Self {
            items,
            idx,
            input: Default::default(),
            result_cache: Default::default(),
        }
    }
    pub fn with_title(self, title: impl Into<String>) -> MsgBuilder<Self> {
        MsgBuilder::new(self, title.into())
    }
}

impl Msg for FzfFinder {
    type Result = Option<usize>;

    fn match_key_event(&mut self, kv: &Key) -> Route {
        let result = self.input.match_key_event(kv);
        if matches!(result, Route::Keep) {
            self.result_cache = self
                .idx
                .search(&self.input.buffer)
                .into_iter()
                .map(|hit| hit.id)
                .collect();
        }
        result
    }

    fn send(self, tx: Sender<Self::Result>) {
        tx.send(self.result_cache.first().copied()).unwrap()
    }

    fn render(&self, f: &mut Frame, area: Rect, block: Block, is_focused: bool) {
        let areas: [Rect; 2] = {
            use ratatui::layout::{Constraint, Layout, Spacing::Overlap};
            let ipt_h = self.input.size().1;
            Layout::vertical([Constraint::Fill(1), Constraint::Length(ipt_h + 2)])
                .spacing(Overlap(1))
                .areas(area)
        };
        self.input.render(f, areas[1], block.clone(), is_focused);

        let widget = Paragraph::new(if self.result_cache.is_empty() {
            Text::from_iter(self.items.iter().map(|s| s.as_str()))
        } else {
            Text::from_iter(
                self.result_cache
                    .iter()
                    .map(|idx| self.items[*idx].as_str()),
            )
        })
        .block(block);
        f.render_widget(widget, areas[0]);
    }

    fn size(&self) -> (u16, u16) {
        let ipt_size = self.input.size();
        (ipt_size.0, ipt_size.1 + 1 + self.items.len().min(3) as u16)
    }
}
