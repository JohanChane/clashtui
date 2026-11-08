//! 页面运行时。Host 持有页面、组件状态、任务集和 Tick 计划。

mod split;

use std::future::Future;
use std::time::{Duration, Instant};

use ratatui::prelude::{Frame, Rect};

pub(in crate::tui) use split::{Side, Split};

use crate::tui::event::Event;

type CallBack<P> = Box<dyn FnOnce(&mut P) + Send>;
pub(in crate::tui) type FutureSet<P> = tokio::task::JoinSet<CallBack<P>>;

/// 页面：有自己的 Action、按键作用域和事件处理。
pub trait Page: Sized + 'static {
    type Action: Copy;
    /// 渲染用的组件状态，如 ListState。
    type State: Default;
    const TITLE: &'static str;
    /// 显示期间的 Tick 周期。None 表示不需要定时刷新。
    const TICK: Option<Duration> = None;

    fn on_event(&mut self, _event: &Event, _cx: &mut Cx<Self>) {}
    fn on_action(&mut self, action: Self::Action, cx: &mut Cx<Self>, state: &mut Self::State);
    /// 单页面标签页总是传 focused = true。
    fn render(&self, f: &mut Frame, area: Rect, state: &mut Self::State, focused: bool);
}

/// 处理函数的上下文。Host 在每次调用处理函数前创建。
pub struct Cx<'a, P> {
    tasks: &'a mut FutureSet<P>,
    /// 所在标签页是否显示。页面通过 is_shown 读取；当前没有页面调用它。
    #[allow(dead_code)]
    shown: bool,
}

impl<P: 'static> Cx<'_, P> {
    pub fn spawn(&mut self, task: impl Future<Output = CallBack<P>> + Send + 'static) {
        self.tasks.spawn(task);
    }

    #[allow(dead_code)]
    pub fn is_shown(&self) -> bool {
        self.shown
    }
}

/// 沿用现有的后缀写法：async { .. }.spawn_at(cx)
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

/// 页面运行时。
pub struct Host<P: Page> {
    page: P,
    state: P::State,
    /// on_event 派生的任务。下一次 Tick 等它们全部完成。
    event_tasks: FutureSet<P>,
    /// on_action 派生的任务（弹窗、测速等）。不影响 Tick。
    action_tasks: FutureSet<P>,
    shown: bool,
    next_tick: Option<Instant>,
}

impl<P: Page + Default> Default for Host<P> {
    /// 只构造，不派生任务。Init 由 Hub 投递。
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
    /// 投递生命周期事件或广播事件。
    pub fn deliver(&mut self, event: &Event) {
        if let Event::Shown | Event::Hidden = event {
            self.shown = *event == Event::Shown;
            self.next_tick = None;
        }
        let mut cx = Cx {
            tasks: &mut self.event_tasks,
            shown: self.shown,
        };
        self.page.on_event(event, &mut cx);
    }

    pub fn handle_action(&mut self, action: P::Action) {
        let mut cx = Cx {
            tasks: &mut self.action_tasks,
            shown: self.shown,
        };
        self.page.on_action(action, &mut cx, &mut self.state);
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect, focused: bool) {
        self.page.render(f, area, &mut self.state, focused);
    }

    /// 每帧调用：应用已完成任务的回调，再检查 Tick 是否到期。
    pub fn sync(&mut self, now: Instant) {
        for tasks in [&mut self.event_tasks, &mut self.action_tasks] {
            while let Some(done) = tasks.try_join_next() {
                // 只有任务被取消或 panic 时才会出错，与现状相同
                done.unwrap()(&mut self.page);
            }
        }
        self.poll_tick(now);
    }

    /// 固定延迟：显示中、且 on_event 派生的任务全部完成后，再过 TICK 发出 Tick。
    /// 用两个任务集而不是给任务打标签：Tick 只需看 event_tasks 是否为空。
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
    async fn hidden_pages_receive_broadcasts_without_ticking() {
        use crate::tui::event::{CoreEvent, FileEvent};

        let now = Instant::now();
        let mut host = Host::<Probe>::default();
        host.deliver(&Event::Shown);
        host.sync(now);
        host.deliver(&Event::Hidden);
        host.deliver(&Event::Core(CoreEvent::ConnectionsClosed));
        host.deliver(&Event::File(FileEvent::ProfilesChanged));
        host.sync(now + Duration::from_secs(10));

        assert_eq!(host.page().broadcasts, 2);
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
