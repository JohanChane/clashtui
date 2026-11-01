pub use super::tab::*;
use crate::tui::TuiWidget;
use ratatui::prelude::{Frame, Rect};

pub trait DualTabContent: BasicTabContent
where
    Self: Sized,
{
    type Mate: DualTabContentMate<Mate = Self>;
    fn command_completed(
        &mut self,
        _event: &crate::tui::commands::CommandEvent,
        _tasks: &mut FutureSet<(Self, Self::Mate)>,
    ) {
    }

    /// This function will be called when creating an instance(`DualTab<C1,C2>::default()`)
    fn init(&mut self, task_set: &mut FutureSet<(Self, Self::Mate)>, state: &mut Self::State);

    /// If true, switch to Mate
    fn handle_action(
        &mut self,
        action: Self::Action,
        task_set: &mut FutureSet<(Self, Self::Mate)>,
        state: &mut Self::State,
        commands: &mut crate::tui::commands::Commands,
    ) -> bool;

    fn render(&self, f: &mut Frame, area: Rect, state: &mut Self::State, is_focused: bool);
}
/// Helper trait for [`DualTabContent`]
pub trait DualTabContentMate: BasicTabContent
where
    Self: Sized,
{
    type Mate: DualTabContent<Mate = Self>;

    /// This function will be called when creating an instance(`DualTab<C1,C2>::default()`)
    fn init(&mut self, task_set: &mut FutureSet<(Self::Mate, Self)>, state: &mut Self::State);

    /// If true, switch to Mate
    fn handle_action(
        &mut self,
        action: Self::Action,
        task_set: &mut FutureSet<(Self::Mate, Self)>,
        state: &mut Self::State,
        commands: &mut crate::tui::commands::Commands,
    ) -> bool;

    fn render(&self, f: &mut Frame, area: Rect, state: &mut Self::State, is_focused: bool);
}

pub struct DualTab<C1, C2>
where
    C1: DualTabContent<Mate = C2>,
    C2: DualTabContentMate<Mate = C1>,
{
    content: (C1, C2),
    state: (C1::State, C2::State),
    tasks: FutureSet<(C1, C2)>,
    is_focus_on_c1: bool,
}

impl<C1, C2> Default for DualTab<C1, C2>
where
    C1: DualTabContent<Mate = C2> + Default,
    C1::State: Default,
    C2: DualTabContentMate<Mate = C1> + Default,
    C2::State: Default,
{
    fn default() -> Self {
        let mut content: (C1, C2) = Default::default();
        let mut state: (C1::State, C2::State) = Default::default();
        let mut tasks = Default::default();
        content.0.init(&mut tasks, &mut state.0);
        content.1.init(&mut tasks, &mut state.1);
        Self {
            content,
            state,
            tasks,
            is_focus_on_c1: true,
        }
    }
}

impl<C1, C2> DualTab<C1, C2>
where
    C1: DualTabContent<Mate = C2>,
    C2: DualTabContentMate<Mate = C1>,
{
    pub fn is_focused_on_left(&self) -> bool {
        self.is_focus_on_c1
    }

    pub fn handle_left_action(
        &mut self,
        action: C1::Action,
        commands: &mut crate::tui::commands::Commands,
    ) {
        debug_assert!(self.is_focus_on_c1);
        if self
            .content
            .0
            .handle_action(action, &mut self.tasks, &mut self.state.0, commands)
        {
            self.is_focus_on_c1 = false;
        }
    }
    pub fn handle_right_action(
        &mut self,
        action: C2::Action,
        commands: &mut crate::tui::commands::Commands,
    ) {
        debug_assert!(!self.is_focus_on_c1);
        if self
            .content
            .1
            .handle_action(action, &mut self.tasks, &mut self.state.1, commands)
        {
            self.is_focus_on_c1 = true;
        }
    }
}

impl<C1, C2> TuiWidget for DualTab<C1, C2>
where
    C1: DualTabContent<Mate = C2>,
    C2: DualTabContentMate<Mate = C1>,
{
    fn render(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        use ratatui::layout::{Constraint::Ratio, Layout};

        let cons = if self.is_focus_on_c1 {
            [Ratio(7, 10), Ratio(3, 10)]
        } else {
            [Ratio(3, 10), Ratio(7, 10)]
        };
        let hori = Layout::horizontal(cons).split(area);

        self.content
            .0
            .render(f, hori[0], &mut self.state.0, self.is_focus_on_c1);
        self.content
            .1
            .render(f, hori[1], &mut self.state.1, !self.is_focus_on_c1);
    }

    fn command_done(&mut self, event: &crate::tui::commands::CommandEvent) {
        self.content.0.command_completed(event, &mut self.tasks);
    }

    fn sync(&mut self) {
        while let Some(f) = self.tasks.try_join_next() {
            f.unwrap()(&mut self.content)
        }
    }
}
