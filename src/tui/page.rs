//! Page runtime. Host owns the page, widget state, task sets, and Tick schedule.

mod split;

use std::future::Future;
use std::time::{Duration, Instant};

use ratatui::prelude::{Frame, Rect};

pub(in crate::tui) use split::{Side, Split};

use crate::tui::event::Event;

type CallBack<P> = Box<dyn FnOnce(&mut P) + Send>;
pub(in crate::tui) type FutureSet<P> = tokio::task::JoinSet<CallBack<P>>;

/// A page with its own Action, key scope, and event handling.
pub trait Page: Sized + 'static {
    type Action: Copy;
    /// Widget state used for rendering, such as ListState.
    type State: Default;
    const TITLE: &'static str;
    /// Tick interval while visible. None disables periodic refreshes.
    const TICK: Option<Duration> = None;

    fn on_event(&mut self, _event: &Event, _cx: &mut Cx<Self>) {}
    fn on_action(&mut self, action: Self::Action, cx: &mut Cx<Self>, state: &mut Self::State);
    /// Single-page tabs always pass focused = true.
    fn render(&self, f: &mut Frame, area: Rect, state: &mut Self::State, focused: bool);
}

/// Handler context, created by Host before each handler call.
pub struct Cx<'a, P> {
    tasks: &'a mut FutureSet<P>,
}

impl<P: 'static> Cx<'_, P> {
    pub fn spawn(&mut self, task: impl Future<Output = CallBack<P>> + Send + 'static) {
        self.tasks.spawn(task);
    }
}

/// Preserve the existing suffix syntax: async { .. }.spawn_at(cx)
pub trait FutureSetExt<P: 'static>: Future<Output = CallBack<P>> + Send + Sized + 'static {
    fn spawn_at(self, cx: &mut Cx<P>) {
        cx.spawn(self);
    }
}

impl<F, P> FutureSetExt<P> for F
where
    F: Future<Output = CallBack<P>> + Send + 'static,
    P: 'static,
{
}

/// Wrap a closure to [`CallBack`], used to wrap the return function of a future.
pub fn wrapper<P>(f: impl FnOnce(&mut P) + 'static + Send) -> CallBack<P> {
    Box::new(f)
}

pub fn do_nothing<P>() -> CallBack<P> {
    wrapper(|_| ())
}

/// Page runtime.
pub struct Host<P: Page> {
    page: P,
    state: P::State,
    /// Tasks spawned by on_event. The next Tick waits for all of them to finish.
    event_tasks: FutureSet<P>,
    /// Tasks spawned by on_action (popups, latency tests, etc.). They do not affect Tick.
    action_tasks: FutureSet<P>,
    shown: bool,
    next_tick: Option<Instant>,
}

impl<P: Page + Default> Default for Host<P> {
    /// Construct without spawning tasks. Hub delivers Init.
    fn default() -> Self {
        Self {
            page: P::default(),
            state: P::State::default(),
            event_tasks: FutureSet::new(),
            action_tasks: FutureSet::new(),
            shown: false,
            next_tick: None,
        }
    }
}

impl<P: Page> Host<P> {
    /// Deliver a lifecycle or broadcast event.
    pub fn deliver(&mut self, event: &Event) {
        if let Event::Shown | Event::Hidden = event {
            self.shown = *event == Event::Shown;
            self.next_tick = None;
        }
        if !self.shown && matches!(event, Event::Core(_) | Event::File(_)) {
            return;
        }
        let mut cx = Cx {
            tasks: &mut self.event_tasks,
        };
        self.page.on_event(event, &mut cx);
    }

    pub fn handle_action(&mut self, action: P::Action) {
        let mut cx = Cx {
            tasks: &mut self.action_tasks,
        };
        self.page.on_action(action, &mut cx, &mut self.state);
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect, focused: bool) {
        self.page.render(f, area, &mut self.state, focused);
    }

    /// Called each frame: apply completed task callbacks, then check whether Tick is due.
    pub fn sync(&mut self, now: Instant) {
        for tasks in [&mut self.event_tasks, &mut self.action_tasks] {
            while let Some(done) = tasks.try_join_next() {
                // As before, only cancellation or panic causes a task error.
                done.unwrap()(&mut self.page);
            }
        }
        self.poll_tick(now);
    }

    /// Fixed delay: while visible, wait TICK after all on_event tasks finish before delivering Tick.
    /// Use two task sets instead of tagging tasks: Tick only checks whether event_tasks is empty.
    fn poll_tick(&mut self, now: Instant) {
        let Some(period) = P::TICK else { return };
        if !self.shown || !self.event_tasks.is_empty() {
            self.next_tick = None;
            return;
        }
        match self.next_tick {
            None => self.next_tick = Some(now + period),
            Some(due) if now >= due => {
                self.next_tick = None;
                self.deliver(&Event::Tick);
            }
            Some(_) => {}
        }
    }

    #[cfg(test)]
    pub fn page(&self) -> &P {
        &self.page
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Probe {
        ticks: usize,
        broadcasts: usize,
        completed: bool,
    }

    impl Page for Probe {
        type Action = ();
        type State = ();
        const TITLE: &'static str = "Probe";
        const TICK: Option<Duration> = Some(Duration::from_secs(1));

        fn on_event(&mut self, event: &Event, _: &mut Cx<Self>) {
            match event {
                Event::Tick => self.ticks += 1,
                Event::Core(_) | Event::File(_) => self.broadcasts += 1,
                _ => {}
            }
        }

        fn on_action(&mut self, _: (), _: &mut Cx<Self>, _: &mut ()) {}

        fn render(&self, _: &mut Frame, _: Rect, _: &mut (), _: bool) {}
    }

    #[tokio::test]
    async fn ticks_only_while_shown() {
        let t0 = Instant::now();
        let mut host = Host::<Probe>::default();
        host.sync(t0 + Duration::from_secs(5));
        assert_eq!(host.page().ticks, 0);

        host.deliver(&Event::Shown);
        host.sync(t0);
        host.sync(t0 + Duration::from_secs(1));
        assert_eq!(host.page().ticks, 1);

        host.deliver(&Event::Hidden);
        host.sync(t0 + Duration::from_secs(9));
        assert_eq!(host.page().ticks, 1);
    }

    #[tokio::test]
    async fn hidden_pages_ignore_broadcasts_and_ticks() {
        use crate::tui::event::{CoreEvent, FileEvent};

        let now = Instant::now();
        let mut host = Host::<Probe>::default();
        host.deliver(&Event::Shown);
        host.deliver(&Event::Core(CoreEvent::ConnectionsClosed));
        assert_eq!(host.page().broadcasts, 1);

        host.sync(now);
        host.deliver(&Event::Hidden);
        host.deliver(&Event::Core(CoreEvent::ConnectionsClosed));
        host.deliver(&Event::File(FileEvent::ProfilesChanged));
        host.sync(now + Duration::from_secs(10));

        assert_eq!(host.page().broadcasts, 1);
        assert_eq!(host.page().ticks, 0);
    }

    #[tokio::test]
    async fn event_tasks_delay_ticks_but_action_tasks_do_not() {
        let now = Instant::now();
        let mut host = Host::<Probe>::default();
        host.deliver(&Event::Shown);
        host.sync(now);

        let (finish, wait) = tokio::sync::oneshot::channel::<()>();
        host.event_tasks.spawn(async move {
            wait.await.unwrap();
            wrapper(|page: &mut Probe| page.completed = true)
        });
        host.action_tasks.spawn(std::future::pending());
        let completed_at = now + Duration::from_secs(10);
        host.sync(completed_at);
        assert_eq!(host.page().ticks, 0);

        finish.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            while !host.page().completed {
                tokio::task::yield_now().await;
                host.sync(completed_at);
            }
        })
        .await
        .unwrap();
        assert_eq!(host.page().ticks, 0);
        host.sync(completed_at + Duration::from_millis(999));
        assert_eq!(host.page().ticks, 0);
        host.sync(completed_at + Duration::from_secs(1));
        assert_eq!(host.page().ticks, 1);
        assert!(!host.action_tasks.is_empty());
    }
}
