//! Continuously probe service and API status, combine them into CoreStatus, and broadcast changes.

use std::sync::RwLock;
use std::time::{Duration, Instant};

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::config::CONFIG;
use crate::config::CoreType;
use crate::scripts::ServiceState;
static SERVICE_STATUS: RwLock<[Option<ServiceState>; 2]> = RwLock::new([None; 2]);
pub(super) fn set_service_status(core: CoreType, status: Option<ServiceState>) {
    SERVICE_STATUS.write().unwrap()[core.index()] = status;
}
use crate::functions::restful::core_detect;
use crate::tui::event::{CoreEvent, CoreStatus, Event, emit};

const PERIOD: Duration = Duration::from_secs(2);
/// Confirmation window after a service command: probe rapidly without treating Unreachable as final.
const SETTLE: Duration = Duration::from_secs(3);
const SETTLE_PERIOD: Duration = Duration::from_millis(300);

/// Latest core status. Only the run task writes it.
static STATUS: RwLock<CoreStatus> = RwLock::new(CoreStatus::Unknown);

/// Core status snapshot, updated before StatusChanged is emitted.
pub fn status() -> CoreStatus {
    STATUS.read().unwrap().clone()
}

/// Reasons to wake the background task.
enum Wake {
    /// A service command ran: publish Unknown and enter the confirmation window.
    Service,
    /// Runtime configuration changed: probe immediately.
    Config,
}

/// Foreground handle for CoreMonitor, owned by Hub. The run task probes in the background.
pub(in crate::tui) struct CoreMonitor {
    wake: UnboundedSender<Wake>,
}

impl CoreMonitor {
    pub fn start() -> Self {
        let (wake, woken) = unbounded_channel();
        tokio::spawn(run(woken));
        Self { wake }
    }

    /// Hub forwards every broadcast here. Only the run task writes status, avoiding races with active probes.
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
        // During confirmation, treat a running service with an unreachable API as still starting; do not publish.
        if !(settling && matches!(status, CoreStatus::Unreachable { .. })) {
            publish(status);
        }
        let period = if settling { SETTLE_PERIOD } else { PERIOD };
        tokio::select! {
            _ = tokio::time::sleep(period) => {}
            Some(reason) = woken.recv() => {
                if let Wake::Service = reason {
                    // A service command invalidates the previous result; pages immediately stop using stale data.
                    // The next transition from Unknown refreshes pages even after a quick restart.
                    publish(CoreStatus::Unknown);
                    settle_until = Some(Instant::now() + SETTLE);
                }
            }
        }
    }
}

/// Publish only on transitions; do nothing if the status is unchanged.
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

/// Run on a blocking thread. The mismatch guard in request() does not affect /version.
/// Probe the API first; use the command scheduler's service snapshot on failure.
fn probe() -> CoreStatus {
    match core_detect::probe() {
        Ok((running, _)) if running != CONFIG.core_type() => CoreStatus::Mismatch { running },
        Ok((_, version)) => CoreStatus::Ready { version },
        Err(error) => match SERVICE_STATUS.read().unwrap()[CONFIG.core_type().index()] {
            Some(ServiceState::Inactive | ServiceState::Missing) => CoreStatus::Stopped,
            // The service is running, or its state is unknown (e.g. system-level launchd on macOS).
            _ => CoreStatus::Unreachable {
                reason: error.to_string(),
            },
        },
    }
}
