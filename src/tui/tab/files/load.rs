//! One actual load per pane, across both mutations and visible lifetimes.
use crate::tui::job::Job;
use std::task::{Context, Poll};

pub(super) struct LoadSlot<T> {
    active: bool,
    attempted_revision: Option<u64>,
    /// Revision captured when the current job started.
    revision: u64,
    job: Option<Job<T>>,
    completed: Option<Result<T, String>>,
}
impl<T> Default for LoadSlot<T> {
    fn default() -> Self {
        Self {
            active: false,
            attempted_revision: None,
            revision: 0,
            job: None,
            completed: None,
        }
    }
}
impl<T: Send + 'static> LoadSlot<T> {
    pub fn enter(&mut self) {
        self.active = true;
        self.attempted_revision = None;
    }
    pub fn leave(&mut self) {
        self.active = false;
        self.attempted_revision = None;
        if let Some(job) = &mut self.job {
            job.accept_result = false;
        }
    }
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }
    pub fn advance(
        &mut self,
        revision: u64,
        load: impl FnOnce() -> Result<T, String> + Send + 'static,
    ) {
        if self.active && !self.busy() && self.attempted_revision != Some(revision) {
            self.revision = revision;
            self.job = Some(Job::start(load));
        }
    }
    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if self.completed.is_some() {
            return Poll::Ready(());
        }
        let Some(job) = &mut self.job else {
            return Poll::Pending;
        };
        match job.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => {
                self.completed =
                    Some(result.unwrap_or_else(|e| Err(format!("File load failed: {e}"))));
                Poll::Ready(())
            }
        }
    }
    pub fn finish(&mut self) -> Option<Result<T, String>> {
        let result = self.completed.take()?;
        let job = self.job.take().unwrap();
        if self.active && job.accept_result {
            // Errors count as an attempt too; only a new revision or enter retries them.
            self.attempted_revision = Some(self.revision);
            Some(result)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::poll_fn;

    #[tokio::test]
    async fn mutation_during_loading_is_followed_without_another_notification() {
        let mut slot = LoadSlot::default();
        slot.enter();
        let (release, wait) = std::sync::mpsc::channel();
        slot.advance(2, move || {
            wait.recv().unwrap();
            Ok(2)
        });
        slot.advance(3, || panic!("must not overlap actual loads"));
        release.send(()).unwrap();
        poll_fn(|cx| slot.poll(cx)).await;
        assert_eq!(slot.finish(), Some(Ok(2)));
        slot.advance(3, || Ok(3));
        assert!(slot.busy());
        poll_fn(|cx| slot.poll(cx)).await;
        assert_eq!(slot.finish(), Some(Ok(3)));
        slot.advance(3, || panic!("no mutation, no new load"));
        assert!(!slot.busy());
    }

    #[tokio::test]
    async fn leave_and_return_waits_for_the_old_job_but_never_accepts_it() {
        let mut slot = LoadSlot::default();
        slot.enter();
        let (release, wait) = std::sync::mpsc::channel();
        slot.advance(4, move || {
            wait.recv().unwrap();
            Ok(4)
        });
        slot.leave();
        slot.enter();
        slot.advance(5, || panic!("old job still occupies the slot"));
        release.send(()).unwrap();
        poll_fn(|cx| slot.poll(cx)).await;
        assert!(slot.finish().is_none());
        slot.advance(5, || Ok(5));
        poll_fn(|cx| slot.poll(cx)).await;
        assert_eq!(slot.finish(), Some(Ok(5)));
        slot.advance(6, || Ok(6));
        slot.leave();
        poll_fn(|cx| slot.poll(cx)).await;
        assert!(slot.finish().is_none());
        slot.advance(7, || panic!("hidden or quitting must not reload"));
        assert!(!slot.busy());
    }

    #[tokio::test]
    async fn failed_and_panicked_loads_retry_only_after_change_or_enter() {
        let mut slot = LoadSlot::<()>::default();
        slot.enter();
        slot.advance(1, || Err("unreadable".into()));
        poll_fn(|cx| slot.poll(cx)).await;
        assert!(slot.finish().unwrap().is_err());
        slot.advance(1, || panic!("failed revision must not spin"));
        assert!(!slot.busy());
        slot.advance(2, || panic!("loader crashed"));
        poll_fn(|cx| slot.poll(cx)).await;
        assert!(
            slot.finish()
                .unwrap()
                .unwrap_err()
                .contains("File load failed")
        );
        slot.advance(2, || panic!("panicked revision must not spin"));
        assert!(!slot.busy());
        slot.leave();
        slot.enter();
        slot.advance(2, || Ok(()));
        poll_fn(|cx| slot.poll(cx)).await;
        assert_eq!(slot.finish(), Some(Ok(())));
    }
}
