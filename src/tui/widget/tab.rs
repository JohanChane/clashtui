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

use crate::tui::{Key, TuiWidget};
use ratatui::prelude::{Frame, Rect};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct KeyCombo(pub Vec<Key>);

impl std::ops::Deref for KeyCombo {
    type Target = [Key];
    fn deref(&self) -> &[Key] {
        &self.0
    }
}

pub trait BasicTabContent: 'static {
    type Key: for<'a> TryFrom<&'a Key, Error = ()> + Copy;
    type State;

    const TITLE: &str;

    fn all_shortcuts() -> &'static [(KeyCombo, Self::Key, &'static str)] {
        &[]
    }

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
    shortcuts: Vec<(KeyCombo, &'static str)>,
}

impl<C> TuiWidget for Tab<C>
where
    C: TabContent,
{
    fn handle_key_event(&mut self, kv: &Key) {
        if let Ok(key) = C::Key::try_from(kv) {
            self.content
                .handle_key_event(key, &mut self.tasks, &mut self.state)
        }
    }

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
        let shortcuts: Vec<(KeyCombo, &'static str)> = C::all_shortcuts()
            .iter()
            .map(|(combo, _, desc)| (combo.clone(), *desc))
            .collect();
        Self {
            content,
            state,
            tasks,
            shortcuts,
        }
    }
}

impl<C: TabContent> Tab<C> {
    pub fn shortcuts(&self) -> &[(KeyCombo, &'static str)] {
        &self.shortcuts
    }

    pub fn dispatch_shortcut(&mut self, seq: &[Key]) {
        for (s, key, _) in C::all_shortcuts() {
            if &**s == seq {
                self.content
                    .handle_key_event(*key, &mut self.tasks, &mut self.state);
                return;
            }
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

/// Await a blocking task, turning a [`tokio::task::JoinError`] into a
/// displayable error instead of panicking.
///
/// Used in place of `spawn_blocking(..).await.unwrap()` so that a cancelled or
/// panicked background task (for example while the core API is unreachable)
/// surfaces as a TUI error state and the app keeps retrying, rather than
/// unwrapping and aborting the process.
pub async fn join_blocking<T, E>(handle: tokio::task::JoinHandle<Result<T, E>>) -> Result<T, String>
where
    E: std::fmt::Display,
{
    match handle.await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(e)) => Err(e.to_string()),
        Err(e) => Err(format!("background task failed: {e}")),
    }
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

#[cfg(test)]
mod tests {
    use super::join_blocking;

    #[tokio::test]
    async fn join_blocking_passes_through_ok() {
        let handle = tokio::spawn(async { Ok::<u8, &'static str>(7) });
        assert_eq!(join_blocking(handle).await.unwrap(), 7);
    }

    #[tokio::test]
    async fn join_blocking_passes_through_err() {
        let handle = tokio::spawn(async { Err::<u8, &'static str>("nope") });
        assert_eq!(join_blocking(handle).await.unwrap_err(), "nope");
    }

    #[tokio::test]
    async fn join_blocking_surfaces_join_error_instead_of_panicking() {
        let handle = tokio::spawn(async {
            std::future::pending::<()>().await;
            Ok::<u8, &'static str>(1)
        });
        handle.abort();
        let err = join_blocking(handle).await.unwrap_err();
        assert!(err.contains("background task failed"));
    }
}
