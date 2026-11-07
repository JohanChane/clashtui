//! One actual task, owned independently of whether its result may still be accepted.
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::task::{JoinError, JoinHandle};

pub(in crate::tui) struct Job<T> {
    task: JoinHandle<Result<T, String>>,
    pub(in crate::tui) accept_result: bool,
}

impl<T: Send + 'static> Job<T> {
    pub(in crate::tui) fn start(work: impl FnOnce() -> Result<T, String> + Send + 'static) -> Self {
        Self {
            task: tokio::task::spawn_blocking(work),
            accept_result: true,
        }
    }

    /// `JoinError` stays unformatted. Each slot maps it to its own text.
    pub(in crate::tui) fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Result<T, String>, JoinError>> {
        Pin::new(&mut self.task).poll(cx)
    }
}
