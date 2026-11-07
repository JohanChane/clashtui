use super::{Instance, Msg, Prompt, Wrapped};
use tokio::sync::oneshot::{Receiver, channel};

pub struct MsgBuilder<C> {
    content: C,
    title: String,
    prompt: Option<String>,
}

impl<C: Msg<Result = R> + Send + 'static, R: Send + 'static> MsgBuilder<C> {
    pub fn new(content: C, title: String) -> Self {
        Self {
            content,
            title,
            prompt: None,
        }
    }

    pub fn with_prompt(self, prompt: String) -> Self {
        Self {
            prompt: Some(prompt),
            ..self
        }
    }

    fn build(self) -> (Wrapped, Receiver<R>, String) {
        let (tx, rx) = channel();
        let Self {
            content,
            title,
            prompt,
        } = self;
        let description = match (content.is_notification(), prompt.as_deref()) {
            (true, Some(prompt)) => format!("{title}: {prompt}"),
            _ => title.clone(),
        };
        let cell = Instance {
            content,
            title,
            prompt: prompt.map(Prompt::new),
            tx,
            is_focus_prompt: false,
        };
        (Box::new(cell), rx, description)
    }

    /// Use in App's local message scope or on a blocking worker thread.
    pub fn build_and_send(self) -> Receiver<R> {
        let (message, receiver, description) = self.build();
        if super::send_blocking(message).is_err() {
            log::warn!("Popup delivery closed: {description}");
        }
        receiver
    }

    /// Await queue capacity without blocking a runtime thread, then await the returned reply.
    pub async fn build_and_send_async(self) -> Receiver<R> {
        let (message, receiver, description) = self.build();
        if super::send_async(message).await.is_err() {
            log::warn!("Popup delivery closed: {description}");
        }
        receiver
    }
}

#[cfg(test)]
mod tests {
    use crate::tui::popmsg::prelude::Confirm;

    #[test]
    fn delivery_logs_keep_operation_feedback_but_exclude_interactive_content() {
        let (_, _, description) = Confirm::title("Preview".into())
            .with_prompt("private configuration".into())
            .build();
        assert_eq!(description, "Preview");
        let (_, _, description) = Confirm::notification("Update failed".into())
            .with_prompt("connection timed out".into())
            .build();
        assert_eq!(description, "Update failed: connection timed out");
    }
}
