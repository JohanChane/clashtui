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

use crate::tui::binding::DisplayBinding;
use crate::tui::binding::Keymap;
use crate::tui::{Key, TuiWidget};
use ratatui::prelude::{Frame, Rect};

pub trait BasicTabContent: 'static {
    type Key: Copy;
    type State;

    const TITLE: &str;

    /// Return the keymap for this tab content (per-scope Keymap).
    fn keymap() -> &'static Keymap<Self::Key>;

    /// Allow you to do something after one task is done
    fn after_sync(&self, _task_set: &mut FutureSet<Self>) {}

    fn on_enter(&mut self, _task_set: &mut FutureSet<Self>, _state: &mut Self::State) {}
    fn on_leave(&mut self, _task_set: &mut FutureSet<Self>, _state: &mut Self::State) {}
}

type CallBack<C> = Box<dyn FnOnce(&mut C) + Send>;
pub type FutureSet<C> = tokio::task::JoinSet<CallBack<C>>;

pub trait TabContent: BasicTabContent {
    /// This function will be called when creating an instance(`Tab<C>::default()`)
    fn init(&mut self, task_set: &mut FutureSet<Self>, state: &mut Self::State);

    fn handle_key_event(
        &mut self,
        key: Self::Key,
        task_set: &mut FutureSet<Self>,
        state: &mut Self::State,
    );

    fn render(&self, f: &mut Frame, area: Rect, state: &mut Self::State);
}

pub struct Tab<C: TabContent> {
    content: C,
    state: C::State,
    tasks: FutureSet<C>,
    shortcuts: Vec<DisplayBinding>,
}

impl<C> TuiWidget for Tab<C>
where
    C: TabContent,
{
    fn render(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        self.content.render(f, area, &mut self.state);
    }

    fn sync(&mut self) {
        while let Some(f) = self.tasks.try_join_next() {
            // SAFETY: panic happens only when a task is canceled(not gonna to happen) or paniced
            f.unwrap()(&mut self.content);
            self.content.after_sync(&mut self.tasks);
        }
    }

    fn on_enter(&mut self) {
        self.content.on_enter(&mut self.tasks, &mut self.state);
    }

    fn on_leave(&mut self) {
        self.content.on_leave(&mut self.tasks, &mut self.state);
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
        let mut tasks = Default::default();
        content.init(&mut tasks, &mut state);
        let shortcuts: Vec<DisplayBinding> = C::keymap().to_display();
        Self {
            content,
            state,
            tasks,
            shortcuts,
        }
    }
}

impl<C: TabContent> Tab<C> {
    pub fn shortcuts(&self) -> &[DisplayBinding] {
        &self.shortcuts
    }

    pub fn dispatch_by_seq(&mut self, seq: &[Key]) {
        if let Some(action) = C::keymap().find_by_seq(seq) {
            self.content
                .handle_key_event(*action, &mut self.tasks, &mut self.state);
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
