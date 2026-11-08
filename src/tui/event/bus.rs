use std::sync::Mutex;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use super::{Broadcast, Event};

/// 当前 Inbox 的发送端。没有 Inbox 时（CLI、单元测试）emit 为空操作。
static SENDER: Mutex<Option<UnboundedSender<Event>>> = Mutex::new(None);

/// 广播通道的接收端，由 App::serve 持有。
pub struct Inbox(UnboundedReceiver<Event>);

/// 广播一个事件。只入队，可在任意线程调用。
pub fn emit(event: impl Broadcast) {
    let event = event.into();
    log::debug!("emit {event:?}");
    if let Some(sender) = SENDER.lock().unwrap().as_ref() {
        let _ = sender.send(event);
    }
}

impl Inbox {
    /// 安装全局发送端。必须在任何生产者启动前调用。
    pub fn open() -> Self {
        let (sender, receiver) = unbounded_channel();
        *SENDER.lock().unwrap() = Some(sender);
        Self(receiver)
    }

    pub async fn recv(&mut self) -> Option<Event> {
        self.0.recv().await
    }
}

impl Drop for Inbox {
    fn drop(&mut self) {
        *SENDER.lock().unwrap() = None;
    }
}
