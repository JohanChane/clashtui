use super::*;
use crate::tui::Key;
use crate::tui::theme::Theme;
use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Layout};
use ratatui::widgets::{Clear, Paragraph};

/// To erease the type `C` in `Instance<C>`
pub type Wrapped = Box<dyn Wrapper + Send>;

pub trait Wrapper {
    fn handle_key_event(&mut self, kv: &Key) -> Route;
    fn render(&mut self, f: &mut Frame);
    fn send(self: Box<Self>);
}

impl<C: Msg> Wrapper for Instance<C> {
    fn handle_key_event(&mut self, kv: &Key) -> Route {
        if matches!(kv.code, KeyCode::Tab) {
            self.is_focus_prompt = (!self.is_focus_prompt) && self.prompt.is_some();
            return Route::Keep;
        }
        if let Some(prompt) = self.prompt.as_mut()
            && self.is_focus_prompt
        {
            prompt.handle_key_event(kv)
        } else {
            self.content.match_key_event(kv)
        }
    }

    fn render(&mut self, f: &mut Frame) {
        use ratatui::symbols::merge::MergeStrategy;
        let layout = self.content.layout();
        let content_block = match layout {
            PopupLayout::Centered => Block::bordered(),
            PopupLayout::Fullscreen => Block::default(),
        }
        .border_style(Theme::get().popup.border)
        .merge_borders(MergeStrategy::Exact);
        let base_block = content_block.clone().title(self.title.as_str());

        if let Some(prompt) = self.prompt.as_mut() {
            use ratatui::layout::Spacing::Overlap;
            let areas: [Rect; 2] = {
                let (content_width, content_height) = self.content.size();
                let (_prompt_width, prompt_height) = prompt.size();
                let prompt_text_w = prompt
                    .prompt
                    .lines()
                    .map(|l| l.len() as u16)
                    .max()
                    .unwrap_or(0);
                let title_w = self.title.len() as u16;
                let w = content_width.max(prompt_text_w).max(title_w).max(20);
                let h = content_height + prompt_height + 1;
                let area = match layout {
                    PopupLayout::Centered => calc_area_from(w, h, f.area()),
                    PopupLayout::Fullscreen => f.area(),
                };
                f.render_widget(Clear, area);

                let constraints = match layout {
                    PopupLayout::Centered => [
                        Constraint::Fill(1),
                        Constraint::Length(1 + 1 + content_height),
                    ],
                    PopupLayout::Fullscreen => {
                        [Constraint::Length(prompt_height + 2), Constraint::Fill(1)]
                    }
                };
                Layout::vertical(constraints)
                    .spacing(Overlap(1))
                    .areas(area)
            };

            prompt.render(f, areas[0], base_block);
            self.content
                .render(f, areas[1], content_block, !self.is_focus_prompt);
        } else {
            let area = {
                let (width, height) = self.content.size();
                let title_w = self.title.len() as u16;
                let w = width.max(title_w).max(20);
                match layout {
                    PopupLayout::Centered => calc_area_from(w, height, f.area()),
                    PopupLayout::Fullscreen => f.area(),
                }
            };
            f.render_widget(Clear, area);
            self.content.render(f, area, base_block, true);
        }
    }

    fn send(self: Box<Self>) {
        self.content.send(self.tx);
    }
}

pub struct Prompt {
    prompt: String,
    offset: (u16, u16),
}

impl Prompt {
    pub fn new(prompt: String) -> Self {
        Self {
            prompt,
            offset: (0, 0),
        }
    }
    fn handle_key_event(&mut self, kv: &Key) -> Route {
        match kv.code {
            KeyCode::Up => self.offset.0 = self.offset.0.saturating_sub(1),
            KeyCode::Down => self.offset.0 += 1,
            KeyCode::Left => self.offset.1 = self.offset.1.saturating_sub(1),
            KeyCode::Right => self.offset.1 += 1,

            KeyCode::Esc => return Route::Drop,
            _ => {}
        }
        Route::Keep
    }
    fn render(&mut self, f: &mut Frame, area: Rect, block: Block) {
        let widget = Paragraph::new(self.prompt.as_str())
            .block(block)
            .scroll(self.offset);

        f.render_widget(&widget, area);
    }
    fn size(&self) -> (u16, u16) {
        (0, self.prompt.lines().count() as u16)
    }
}

fn calc_area_from(dialog_width: u16, dialog_height: u16, area: Rect) -> Rect {
    // make up for block
    let dialog_width = (dialog_width + 2).max(30).min(area.width.saturating_sub(4));
    let dialog_height = (dialog_height + 2)
        .max(3)
        .min(area.height.saturating_sub(6));

    let width = Constraint::Length(dialog_width);
    let height = Constraint::Length(dialog_height);

    let tmp = Layout::vertical([Constraint::Min(0), height, Constraint::Min(0)]).split(area);
    Layout::horizontal([Constraint::Min(0), width, Constraint::Min(0)]).split(tmp[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::popmsg::prelude::{FzfFinder, Input};
    use ratatui::{Terminal, backend::TestBackend};
    use tokio::sync::oneshot;

    fn instance<C: Msg>(content: C) -> Instance<C> {
        let (tx, _rx) = oneshot::channel();
        Instance {
            content,
            title: "Find Proxy".into(),
            prompt: None,
            is_focus_prompt: false,
            tx,
        }
    }

    fn draw<C: Msg>(popup: &mut Instance<C>, width: u16, height: u16) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                for y in area.y..area.bottom() {
                    frame.render_widget(
                        Paragraph::new("X".repeat(width as usize)),
                        Rect::new(0, y, width, 1),
                    );
                }
                popup.render(frame);
            })
            .unwrap();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| terminal.backend().buffer()[(x, y)].symbol())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn finder_covers_the_frame_and_anchors_results_above_the_input() {
        let mut popup = instance(FzfFinder::new(vec!["alpha".into(), "beta".into()]));
        let rows = draw(&mut popup, 40, 10);
        assert!(!rows.join("\n").contains('X'));
        assert_eq!(rows[6].trim(), "beta");
        assert_eq!(rows[7].trim(), "> alpha");
        assert!(rows[8].contains("Find Proxy"));
        assert!(rows[8].contains("2/2"));
        assert_eq!(rows[9].trim(), ">");

        let rows = draw(&mut popup, 24, 5);
        assert_eq!(rows[1].trim(), "beta");
        assert_eq!(rows[2].trim(), "> alpha");
        assert_eq!(rows[4].trim(), ">");

        popup.handle_key_event(&KeyCode::Char('0').into());
        let rows = draw(&mut popup, 40, 10);
        assert_eq!(rows[7].trim(), "No matches");
        assert!(rows[8].contains("0/2"));
        assert_eq!(rows[9].trim(), "> 0");
        assert!(!rows.join("\n").contains("alpha"));
    }

    #[test]
    fn finder_keeps_the_selection_visible_when_scrolling_and_resizing() {
        let mut popup = instance(FzfFinder::new(
            (0..20).map(|i| format!("node-{i:02}")).collect(),
        ));
        for _ in 0..15 {
            popup.handle_key_event(&KeyCode::Up.into());
        }
        for (width, height) in [(40, 10), (24, 5), (60, 20)] {
            let rows = draw(&mut popup, width, height);
            assert!(rows.iter().any(|row| row.trim() == "> node-15"));
            assert_eq!(rows.last().unwrap().trim(), ">");
        }
        for (width, height) in [(1, 1), (2, 2), (0, 0)] {
            draw(&mut popup, width, height);
        }
    }

    #[test]
    fn ordinary_input_stays_centered_with_its_prompt() {
        let mut popup = instance(Input::new());
        popup.prompt = Some(Prompt::new("Enter a value".into()));
        let rows = draw(&mut popup, 60, 20);
        assert!(rows[0].chars().all(|ch| ch == 'X'));
        assert!(rows[19].chars().all(|ch| ch == 'X'));
        assert!(rows.iter().any(|row| row.contains("Enter a value")));
        assert!(rows.iter().any(|row| row.contains("Find Proxy")));
    }
}
