//! How to use?
//! ``` no_run
//! struct Content {/* */};
//!
//! impl BasicTabContent for Content {/* */}
//! impl TabContent for Content {/* */}
//!
//! struct TheTab(Tab<Content>);
//! new_type_impl_tuiwidget!(TheTab);
//! ```

use crate::tui::TuiWidget;
use crate::tui::business::{Business, BusinessView};
use ratatui::prelude::{Frame, Rect};
use std::sync::{Arc, Weak};
use std::task::{Context, Poll};

pub trait BasicTabContent: 'static + Sized {
    type Action: Copy;
    type State;

    const TITLE: &str;

    fn on_enter(
        &mut self,
        _task_set: &mut FutureSet<Self>,
        _state: &mut Self::State,
        _cx: &mut Business,
    ) {
    }
    fn on_leave(
        &mut self,
        _task_set: &mut FutureSet<Self>,
        _state: &mut Self::State,
        _cx: &mut Business,
    ) {
    }
    fn sync_from(&mut self, _view: BusinessView<'_>, _state: &mut Self::State) {}
    fn needs_animation(&self) -> bool {
        false
    }
}

pub type CallBack<C> = Box<dyn FnOnce(&mut C) + Send>;
pub type TaskResult<C> = Result<Option<CallBack<C>>, tokio::task::JoinError>;

/// Own actual work independently of disposable callbacks for a page's visible lifetime.
pub struct FutureSet<C> {
    tasks: tokio::task::JoinSet<(Weak<()>, CallBack<C>)>,
    replies: Arc<()>,
}

impl<C> Default for FutureSet<C> {
    fn default() -> Self {
        Self {
            tasks: tokio::task::JoinSet::new(),
            replies: Arc::new(()),
        }
    }
}

impl<C: 'static> FutureSet<C> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn spawn(
        &mut self,
        future: impl Future<Output = CallBack<C>> + Send + 'static,
    ) -> tokio::task::AbortHandle {
        let reply = Arc::downgrade(&self.replies);
        self.tasks.spawn(async move { (reply, future.await) })
    }
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
    pub fn invalidate_replies(&mut self) {
        self.replies = Arc::new(());
    }
    pub fn poll_join_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<TaskResult<C>>> {
        self.tasks.poll_join_next(cx).map(|result| {
            result.map(|result| result.map(|(reply, callback)| reply.upgrade().map(|_| callback)))
        })
    }
}

pub trait TabContent: BasicTabContent {
    /// This function will be called when creating an instance(`Tab<C>::default()`)
    fn init(&mut self, task_set: &mut FutureSet<Self>, state: &mut Self::State);

    fn handle_action(
        &mut self,
        action: Self::Action,
        task_set: &mut FutureSet<Self>,
        state: &mut Self::State,
        cx: &mut Business,
    );

    fn render(&self, f: &mut Frame, area: Rect, state: &mut Self::State, view: BusinessView<'_>);
}

pub struct Tab<C: TabContent> {
    content: C,
    state: C::State,
    tasks: FutureSet<C>,
    completed: Option<TaskResult<C>>,
}

impl<C: TabContent> Tab<C> {
    pub fn handle_action(&mut self, action: C::Action, cx: &mut Business) {
        self.content
            .handle_action(action, &mut self.tasks, &mut self.state, cx);
    }
}

impl<C> TuiWidget for Tab<C>
where
    C: TabContent,
{
    fn render(
        &mut self,
        f: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        view: BusinessView<'_>,
    ) {
        self.content.render(f, area, &mut self.state, view);
    }

    fn on_enter(&mut self, cx: &mut Business) {
        self.content.on_enter(&mut self.tasks, &mut self.state, cx);
    }
    fn on_leave(&mut self, cx: &mut Business) {
        self.content.on_leave(&mut self.tasks, &mut self.state, cx);
        self.tasks.invalidate_replies();
    }
    fn sync_from(&mut self, view: BusinessView<'_>) {
        self.content.sync_from(view, &mut self.state);
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
        self.content.needs_animation()
    }
}

impl<C> Default for Tab<C>
where
    C: TabContent + Default,
    C::State: Default,
{
    fn default() -> Self {
        let mut content: C = Default::default();
        let mut state = Default::default();
        let mut tasks = FutureSet::new();
        content.init(&mut tasks, &mut state);
        Self {
            content,
            state,
            tasks,
            completed: None,
        }
    }
}

/// Wrap a closure to [`CallBack`], used to wrap the return function of a future
///
/// e.g.
/// ``` rust,norun
/// let name = "test".to_owned();
/// let task = async {
///     println!("Start {}", name);
///     tokio::time::sleep(std::time::Duration::from_micros(10)).await;
///     wrapper(move |content: &mut Self| {
///         println!("Done {}", name);
///     })
/// };
/// task_set.spawn(task);
/// ```
pub fn wrapper<C>(f: impl FnOnce(&mut C) + 'static + Send) -> CallBack<C> {
    Box::new(f)
}

pub fn do_nothing<C>() -> CallBack<C> {
    wrapper(|_| ())
}

pub trait FutureSetExt<C>: Future<Output = CallBack<C>>
where
    Self: Sized + Send + 'static,
    C: 'static,
{
    fn spawn_at(self, set: &mut FutureSet<C>) {
        set.spawn(self);
    }
}
impl<F, C> FutureSetExt<C> for F
where
    F: Future<Output = CallBack<C>> + Send + 'static,
    C: 'static,
{
}
