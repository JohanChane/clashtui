use ratatui::layout::{Constraint::Ratio, Layout};
use ratatui::prelude::{Frame, Rect};

use super::{Host, Page};
use crate::tui::event::Event;

/// Two-pane container. Deliver events to both panes and actions only to the focused pane.
pub struct Split<L: Page, R: Page> {
    left: Host<L>,
    right: Host<R>,
    focus: Side,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Side {
    #[default]
    Left,
    Right,
}

impl<L: Page + Default, R: Page + Default> Default for Split<L, R> {
    fn default() -> Self {
        Self {
            left: Host::default(),
            right: Host::default(),
            focus: Side::Left,
        }
    }
}

impl<L: Page, R: Page> Split<L, R> {
    pub fn deliver(&mut self, event: &Event) {
        self.left.deliver(event);
        self.right.deliver(event);
    }

    pub fn command_done(&mut self, event: &crate::tui::commands::CommandEvent) {
        self.left.command_done(event);
        self.right.command_done(event);
    }

    pub fn sync(&mut self, now: std::time::Instant) {
        self.left.sync(now);
        self.right.sync(now);
    }

    /// Give the focused pane 7/10 of the space and the other pane 3/10.
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let cons = if self.focus == Side::Left {
            [Ratio(7, 10), Ratio(3, 10)]
        } else {
            [Ratio(3, 10), Ratio(7, 10)]
        };
        let hori = Layout::horizontal(cons).split(area);
        self.left.render(f, hori[0], self.focus == Side::Left);
        self.right.render(f, hori[1], self.focus == Side::Right);
    }

    pub fn left_action(
        &mut self,
        action: L::Action,
        commands: &mut crate::tui::commands::Commands,
    ) {
        self.left.handle_action(action, commands);
    }

    pub fn right_action(
        &mut self,
        action: R::Action,
        commands: &mut crate::tui::commands::Commands,
    ) {
        self.right.handle_action(action, commands);
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        };
    }

    pub fn focus(&self) -> Side {
        self.focus
    }
}
