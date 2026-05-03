pub use super::tab::*;
use crate::tui::TuiWidget;
use crossterm::event::KeyEvent;
use ratatui::prelude::{Frame, Rect};

pub trait DualTabContent: BasicTabContent
where
    Self: Sized,
{
    type Mate: DualTabContentMate<Mate = Self>;

    /// This function will be called when creating an instance(`DualTab<C1,C2>::default()`)
    fn init(&mut self, task_set: &mut FutureSet<(Self, Self::Mate)>, state: &mut Self::State);

    /// If true, switch to Mate
    fn handle_key_event(
        &mut self,
        key: Self::Key,
        task_set: &mut FutureSet<(Self, Self::Mate)>,
        state: &mut Self::State,
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
    fn handle_key_event(
        &mut self,
        key: Self::Key,
        task_set: &mut FutureSet<(Self::Mate, Self)>,
        state: &mut Self::State,
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
    pub fn shortcuts(&self) -> &[(KeyCombo, &'static str)] {
        use std::sync::OnceLock;
        static C1_CACHED: OnceLock<Vec<(KeyCombo, &str)>> = OnceLock::new();
        static C2_CACHED: OnceLock<Vec<(KeyCombo, &str)>> = OnceLock::new();

        if self.is_focus_on_c1 {
            C1_CACHED.get_or_init(|| {
                C1::all_shortcuts()
                    .iter()
                    .map(|(combo, _, desc)| (combo.clone(), *desc))
                    .collect()
            })
        } else {
            C2_CACHED.get_or_init(|| {
                C2::all_shortcuts()
                    .iter()
                    .map(|(combo, _, desc)| (combo.clone(), *desc))
                    .collect()
            })
        }
    }

    pub fn dispatch_shortcut(&mut self, seq: &[KeyEvent]) {
        if self.is_focus_on_c1 {
            for (s, key, _) in C1::all_shortcuts() {
                if &**s == seq {
                    if DualTabContent::handle_key_event(
                        &mut self.content.0,
                        *key,
                        &mut self.tasks,
                        &mut self.state.0,
                    ) {
                        self.is_focus_on_c1 = false;
                    }
                    return;
                }
            }
        } else {
            for (s, key, _) in C2::all_shortcuts() {
                if &**s == seq {
                    if DualTabContentMate::handle_key_event(
                        &mut self.content.1,
                        *key,
                        &mut self.tasks,
                        &mut self.state.1,
                    ) {
                        self.is_focus_on_c1 = true;
                    }
                    return;
                }
            }
        }
    }
}

impl<C1, C2> TuiWidget for DualTab<C1, C2>
where
    C1: DualTabContent<Mate = C2>,
    C2: DualTabContentMate<Mate = C1>,
{
    fn handle_key_event(&mut self, kv: &KeyEvent) {
        if self.is_focus_on_c1 {
            if let Ok(key) = C1::Key::try_from(kv) {
                if DualTabContent::handle_key_event(
                    &mut self.content.0,
                    key,
                    &mut self.tasks,
                    &mut self.state.0,
                ) {
                    self.is_focus_on_c1 = false
                }
            }
        } else {
            if let Ok(key) = C2::Key::try_from(kv) {
                if DualTabContentMate::handle_key_event(
                    &mut self.content.1,
                    key,
                    &mut self.tasks,
                    &mut self.state.1,
                ) {
                    self.is_focus_on_c1 = true
                }
            }
        }
    }

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

    fn sync(&mut self) {
        while let Some(f) = self.tasks.try_join_next() {
            f.unwrap()(&mut self.content)
        }
    }
}
