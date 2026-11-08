//! 常驻探测。把服务状态和 API 状态合成一个 CoreStatus，变化时广播。

use std::sync::RwLock;
use std::time::{Duration, Instant};

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::config::CONFIG;
use crate::functions::command;
use crate::functions::restful::core_detect;
use crate::tui::event::{CoreEvent, CoreStatus, Event, emit};

const PERIOD: Duration = Duration::from_secs(2);
/// 服务命令之后的确认窗口：窗口内快速探测，且不把 Unreachable 当作结论。
const SETTLE: Duration = Duration::from_secs(3);
const SETTLE_PERIOD: Duration = Duration::from_millis(300);

/// 最新的核心状态。只有 run 任务写。
static STATUS: RwLock<CoreStatus> = RwLock::new(CoreStatus::Unknown);

/// 核心状态快照。在 StatusChanged 发出前已更新。
pub fn status() -> CoreStatus {
    STATUS.read().unwrap().clone()
}

/// 唤醒后台任务的原因。
enum Wake {
    /// 执行了服务命令：发布 Unknown，进入确认窗口。
    Service,
    /// 运行时配置变化：立即探测。
    Config,
}

/// CoreMonitor 的前台句柄，由 Hub 持有。探测循环在后台任务 run 中运行。
pub(in crate::tui) struct CoreMonitor {
    wake: UnboundedSender<Wake>,
}

impl CoreMonitor {
    pub fn start() -> Self {
        let (wake, woken) = unbounded_channel();
        tokio::spawn(run(woken));
        Self { wake }
    }

    /// Hub 转发每个广播事件；这里只做转发，状态只由 run 任务写，避免与进行中的探测竞争。
    pub fn on_event(&self, event: &Event) {
        let reason = match event {
            Event::Core(CoreEvent::ServiceCommandDone) => Wake::Service,
            Event::Core(CoreEvent::ConfigChanged) => Wake::Config,
            _ => return,
        };
        let _ = self.wake.send(reason);
    }
}

async fn run(mut woken: UnboundedReceiver<Wake>) {
    let mut settle_until: Option<Instant> = None;
    loop {
        let status = tokio::task::spawn_blocking(probe).await.unwrap();
        let settling = settle_until.is_some_and(|until| Instant::now() < until);
        // 确认窗口内，服务在运行但 API 还不可达，视为仍在启动，不发布
        if !(settling && matches!(status, CoreStatus::Unreachable { .. })) {
            publish(status);
        }
        let period = if settling { SETTLE_PERIOD } else { PERIOD };
        tokio::select! {
            _ = tokio::time::sleep(period) => {}
            Some(reason) = woken.recv() => {
                if let Wake::Service = reason {
                    // 服务刚被操作过，旧结论作废：页面立即停用旧数据。
                    // 之后必有一次 Unknown → 新状态的变化，快速重启也会让页面刷新。
                    publish(CoreStatus::Unknown);
                    settle_until = Some(Instant::now() + SETTLE);
                }
            }
        }
    }
}

/// 边沿触发：状态不变时什么也不做。
fn publish(status: CoreStatus) {
    {
        let mut current = STATUS.write().unwrap();
        if *current == status {
            return;
        }
        *current = status.clone();
    }
    crate::config::set_core_mismatch(matches!(status, CoreStatus::Mismatch { .. }));
    emit(CoreEvent::StatusChanged(status));
}

/// 在阻塞线程中运行。/version 不受 request() 的 mismatch 防护影响。
/// 先查 API：成功就不查服务，不启动进程。失败时才查服务，因为服务没运行时 API 必然不可达。
fn probe() -> CoreStatus {
    match core_detect::probe() {
        Ok((running, _)) if running != CONFIG.core_type() => CoreStatus::Mismatch { running },
        Ok((_, version)) => CoreStatus::Ready { version },
        Err(error) => match command::is_core_service_running() {
            Some(false) => CoreStatus::Stopped,
            // 服务在运行，或无法判断（如 macOS 系统级 launchd）
            _ => CoreStatus::Unreachable {
                reason: error.to_string(),
            },
        },
    }
}
