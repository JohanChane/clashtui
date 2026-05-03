use super::{Instance, Msg, PAIR, Prompt};
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
    pub fn build_and_send(self) -> Receiver<R> {
        let (tx, rx) = channel();

        let Self {
            content,
            title,
            prompt,
        } = self;

        let cell = Instance {
            content,
            title,
            prompt: prompt.map(Prompt::new),
            tx,
            is_focus_prompt: false,
        };

        PAIR.0.send(Box::new(cell)).unwrap();

        rx
    }
}
