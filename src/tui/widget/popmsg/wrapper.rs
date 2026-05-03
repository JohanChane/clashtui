use super::*;
use crate::tui::theme::Theme;
use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Layout};
use ratatui::widgets::{Clear, Paragraph};

/// To erease the type `C` in `Instance<C>`
pub type Wrapped = Box<dyn Wrapper + Send>;

pub trait Wrapper {
    fn handle_key_event(&mut self, kv: &KeyEvent) -> Route;
    fn render(&mut self, f: &mut Frame);
    fn send(self: Box<Self>);
}

impl<C: Msg> Wrapper for Instance<C> {
    fn handle_key_event(&mut self, kv: &KeyEvent) -> Route {
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
        let base_block = Block::bordered()
            .border_style(Theme::get().popup.block)
            .title(self.title.as_str());

        if let Some(prompt) = self.prompt.as_mut() {
            use ratatui::symbols::{border, line::NORMAL};
            use ratatui::widgets::Borders;
            let base_block = base_block.border_set(border::Set {
                bottom_left: NORMAL.vertical_right,
                bottom_right: NORMAL.vertical_left,
                ..border::PLAIN
            });
            let content_block = Block::bordered()
                .borders(Borders::ALL & !Borders::TOP)
                .border_style(Theme::get().popup.block);

            let areas = {
                let (content_width, content_height) = self.content.size();
                let (_prompt_width, prompt_height) = prompt.size();
                let prompt_text_w = prompt
                    .prompt
                    .lines()
                    .map(|l| l.len() as u16)
                    .max()
                    .unwrap_or(0);
                let title_w = self.title.len() as u16;
                let w = content_width
                    .max(prompt_text_w)
                    .max(title_w)
                    .max(20);
                let h = content_height + prompt_height + 1;
                let area = calc_area_from(w, h, f.area());
                f.render_widget(Clear, area);

                Layout::vertical([Constraint::Fill(1), Constraint::Length(1 + content_height)])
                    .split(area)
            };

            prompt.render(f, areas[0], base_block);
            self.content
                .render(f, areas[1], content_block, !self.is_focus_prompt);
        } else {
            let area = {
                let (width, height) = self.content.size();
                let title_w = self.title.len() as u16;
                let w = width.max(title_w).max(20);
                calc_area_from(w, height, f.area())
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
    fn handle_key_event(&mut self, kv: &KeyEvent) -> Route {
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
    let dialog_width = (dialog_width + 2)
        .max(30)
        .min(area.width.saturating_sub(4));
    let dialog_height = (dialog_height + 2)
        .max(3)
        .min(area.height.saturating_sub(6));

    let width = Constraint::Length(dialog_width);
    let height = Constraint::Length(dialog_height);

    let tmp = Layout::vertical([Constraint::Min(0), height, Constraint::Min(0)]).split(area);
    Layout::horizontal([Constraint::Min(0), width, Constraint::Min(0)]).split(tmp[1])[1]
}
