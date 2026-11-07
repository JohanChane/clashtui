pub use super::tab::*;
use crate::tui::TuiWidget;
use crate::tui::business::{Business, BusinessView};
use ratatui::prelude::{Frame, Rect};
use std::task::{Context, Poll};

pub trait DualTabContent: BasicTabContent
where
    Self: Sized,
{
    type Mate: DualTabContentMate<Mate = Self>;

    /// This function will be called when creating an instance(`DualTab<C1,C2>::default()`)
    fn init(&mut self, task_set: &mut FutureSet<(Self, Self::Mate)>, state: &mut Self::State);
    fn on_enter(
        &mut self,
        _tasks: &mut FutureSet<(Self, Self::Mate)>,
        _state: &mut Self::State,
        _cx: &mut Business,
    ) {
    }
    fn on_leave(
        &mut self,
        _tasks: &mut FutureSet<(Self, Self::Mate)>,
        _state: &mut Self::State,
        _cx: &mut Business,
    ) {
    }
    fn sync_from(&mut self, _view: BusinessView<'_>, _state: &mut Self::State) {}

    /// If true, switch to Mate
    fn handle_action(
        &mut self,
        action: Self::Action,
        task_set: &mut FutureSet<(Self, Self::Mate)>,
        state: &mut Self::State,
        cx: &mut Business,
    ) -> bool;

    fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        state: &mut Self::State,
        is_focused: bool,
        view: BusinessView<'_>,
    );
}
/// Helper trait for [`DualTabContent`]
pub trait DualTabContentMate: BasicTabContent
where
    Self: Sized,
{
    type Mate: DualTabContent<Mate = Self>;

    /// This function will be called when creating an instance(`DualTab<C1,C2>::default()`)
    fn init(&mut self, task_set: &mut FutureSet<(Self::Mate, Self)>, state: &mut Self::State);
    fn on_enter(
        &mut self,
        _tasks: &mut FutureSet<(Self::Mate, Self)>,
        _state: &mut Self::State,
        _cx: &mut Business,
    ) {
    }
    fn on_leave(
        &mut self,
        _tasks: &mut FutureSet<(Self::Mate, Self)>,
        _state: &mut Self::State,
        _cx: &mut Business,
    ) {
    }
    fn sync_from(&mut self, _view: BusinessView<'_>, _state: &mut Self::State) {}

    /// If true, switch to Mate
    fn handle_action(
        &mut self,
        action: Self::Action,
        task_set: &mut FutureSet<(Self::Mate, Self)>,
        state: &mut Self::State,
        cx: &mut Business,
    ) -> bool;

    fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        state: &mut Self::State,
        is_focused: bool,
        view: BusinessView<'_>,
    );
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
    completed: Option<TaskResult<(C1, C2)>>,
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
        let mut tasks = FutureSet::new();
        content.0.init(&mut tasks, &mut state.0);
        content.1.init(&mut tasks, &mut state.1);
        Self {
            content,
            state,
            tasks,
            is_focus_on_c1: true,
            completed: None,
        }
    }
}

impl<C1, C2> DualTab<C1, C2>
where
    C1: DualTabContent<Mate = C2>,
    C2: DualTabContentMate<Mate = C1>,
{
    pub(in crate::tui) fn content_mut(&mut self) -> &mut (C1, C2) {
        &mut self.content
    }

    pub fn is_focused_on_left(&self) -> bool {
        self.is_focus_on_c1
    }

    pub fn handle_left_action(&mut self, action: C1::Action, cx: &mut Business) {
        debug_assert!(self.is_focus_on_c1);
        if self
            .content
            .0
            .handle_action(action, &mut self.tasks, &mut self.state.0, cx)
        {
            self.is_focus_on_c1 = false;
        }
    }
    pub fn handle_right_action(&mut self, action: C2::Action, cx: &mut Business) {
        debug_assert!(!self.is_focus_on_c1);
        if self
            .content
            .1
            .handle_action(action, &mut self.tasks, &mut self.state.1, cx)
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
    fn render(
        &mut self,
        f: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        view: BusinessView<'_>,
    ) {
        use ratatui::layout::{Constraint::Ratio, Layout};

        let cons = if self.is_focus_on_c1 {
            [Ratio(7, 10), Ratio(3, 10)]
        } else {
            [Ratio(3, 10), Ratio(7, 10)]
        };
        let hori = Layout::horizontal(cons).split(area);

        self.content
            .0
            .render(f, hori[0], &mut self.state.0, self.is_focus_on_c1, view);
        self.content
            .1
            .render(f, hori[1], &mut self.state.1, !self.is_focus_on_c1, view);
    }

    fn on_enter(&mut self, cx: &mut Business) {
        DualTabContent::on_enter(&mut self.content.0, &mut self.tasks, &mut self.state.0, cx);
        DualTabContentMate::on_enter(&mut self.content.1, &mut self.tasks, &mut self.state.1, cx);
    }
    fn on_leave(&mut self, cx: &mut Business) {
        DualTabContent::on_leave(&mut self.content.0, &mut self.tasks, &mut self.state.0, cx);
        DualTabContentMate::on_leave(&mut self.content.1, &mut self.tasks, &mut self.state.1, cx);
        self.tasks.invalidate_replies();
    }
    fn sync_from(&mut self, view: BusinessView<'_>) {
        DualTabContent::sync_from(&mut self.content.0, view, &mut self.state.0);
        DualTabContentMate::sync_from(&mut self.content.1, view, &mut self.state.1);
    }
    fn poll_tasks(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if self.completed.is_some() {
            return Poll::Ready(());
        }
        match self.tasks.poll_join_next(cx) {
            Poll::Ready(Some(result)) => {
                self.completed = Some(result);
                Poll::Ready(())
            }
            _ => Poll::Pending,
        }
    }
    fn finish_task(&mut self, visible: bool) -> bool {
        match self.completed.take() {
            Some(Ok(Some(callback))) if visible => {
                callback(&mut self.content);
                true
            }
            Some(Err(error)) => {
                crate::tui::popmsg::prelude::Confirm::err(format!(
                    "Background task failed: {error}"
                ));
                true
            }
            _ => false,
        }
    }
    fn has_tasks(&self) -> bool {
        !self.tasks.is_empty() || self.completed.is_some()
    }
    fn needs_animation(&self) -> bool {
        self.content.0.needs_animation() || self.content.1.needs_animation()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::poll_fn;

    #[derive(Default)]
    struct Pane<const LEFT: bool> {
        enters: u32,
        leaves: u32,
        syncs: u32,
    }
    impl<const LEFT: bool> BasicTabContent for Pane<LEFT> {
        type Action = ();
        type State = ();
        const TITLE: &str = "Test pane";
    }
    macro_rules! pane {
        ($trait:ident, $left:literal, $mate:literal) => {
            impl $trait for Pane<$left> {
                type Mate = Pane<$mate>;
                fn init(&mut self, _: &mut FutureSet<(Pane<true>, Pane<false>)>, _: &mut ()) {}
                fn on_enter(
                    &mut self,
                    _: &mut FutureSet<(Pane<true>, Pane<false>)>,
                    _: &mut (),
                    _: &mut Business,
                ) {
                    self.enters += 1;
                }
                fn on_leave(
                    &mut self,
                    _: &mut FutureSet<(Pane<true>, Pane<false>)>,
                    _: &mut (),
                    _: &mut Business,
                ) {
                    self.leaves += 1;
                }
                fn sync_from(&mut self, _: BusinessView<'_>, _: &mut ()) {
                    self.syncs += 1;
                }
                fn handle_action(
                    &mut self,
                    _: (),
                    _: &mut FutureSet<(Pane<true>, Pane<false>)>,
                    _: &mut (),
                    _: &mut Business,
                ) -> bool {
                    true
                }
                fn render(&self, _: &mut Frame, _: Rect, _: &mut (), _: bool, _: BusinessView<'_>) {
                }
            }
        };
    }
    pane!(DualTabContent, true, false);
    pane!(DualTabContentMate, false, true);

    #[test]
    fn both_visible_panes_sync_and_focus_does_not_change_lifecycle() {
        let mut tab = DualTab::<Pane<true>, Pane<false>>::default();
        let mut business = Business::default();
        tab.on_enter(&mut business);
        tab.sync_from(business.view());
        tab.handle_left_action((), &mut business);
        assert!(!tab.is_focused_on_left());
        tab.sync_from(business.view());
        assert_eq!((tab.content.0.enters, tab.content.1.enters), (1, 1));
        assert_eq!((tab.content.0.syncs, tab.content.1.syncs), (2, 2));
        tab.on_leave(&mut business);
        assert_eq!((tab.content.0.leaves, tab.content.1.leaves), (1, 1));
    }

    #[tokio::test]
    async fn reenter_keeps_actual_operations_but_drops_their_old_callbacks() {
        let mut tab = DualTab::<Pane<true>, Pane<false>>::default();
        let mut business = Business::default();
        let (release, wait) = tokio::sync::oneshot::channel();
        let finished = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let work_finished = finished.clone();
        tab.tasks.spawn(async move {
            wait.await.unwrap();
            work_finished.store(true, std::sync::atomic::Ordering::SeqCst);
            wrapper(|_: &mut (Pane<true>, Pane<false>)| panic!("stale callback"))
        });
        tab.on_leave(&mut business);
        tab.on_enter(&mut business);
        assert!(tab.has_tasks());
        release.send(()).unwrap();
        poll_fn(|cx| tab.poll_tasks(cx)).await;
        assert!(!tab.finish_task(true));
        assert!(!tab.has_tasks());
        assert!(finished.load(std::sync::atomic::Ordering::SeqCst));
    }
}
