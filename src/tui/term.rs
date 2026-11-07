use super::utils::raw_mode;
use anyhow::{Result, anyhow, bail};
use std::sync::{
    Arc, Condvar, Mutex, Weak,
    atomic::{AtomicBool, Ordering},
    mpsc as sync_mpsc,
};
use tokio::sync::mpsc;

mod input;
pub(in crate::tui) use input::InputReader;

static COORDINATOR: Mutex<Option<Weak<Bridge>>> = Mutex::new(None);

type Acknowledgement = sync_mpsc::SyncSender<Result<(), String>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OperationId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Suspend,
    Resume,
}

pub(in crate::tui) struct TerminalControl {
    id: OperationId,
    action: Action,
    acknowledgement: Acknowledgement,
}

impl TerminalControl {
    pub(in crate::tui) fn is_suspend(&self) -> bool {
        self.action == Action::Suspend
    }
}

#[derive(Default)]
struct Slot {
    next_id: u64,
    owner: Option<OperationId>,
    waiting_ack: Option<Acknowledgement>,
}

struct Bridge {
    sender: mpsc::Sender<TerminalControl>,
    slot: Mutex<Slot>,
    available: Condvar,
    closing: AtomicBool,
}

impl Bridge {
    fn close(&self) {
        self.closing.store(true, Ordering::Release);
        let mut slot = self.slot.lock().unwrap();
        if let Some(ack) = slot.waiting_ack.take() {
            let _ = ack.try_send(Err("Terminal handoff cancelled".into()));
        }
        self.available.notify_all();
    }

    fn release_owner(&self, id: OperationId) {
        let mut slot = self.slot.lock().unwrap();
        if slot.owner == Some(id) {
            slot.owner = None;
            slot.waiting_ack = None;
            self.available.notify_all();
        }
    }
}

/// Owned by App. Workers request terminal access; only App changes terminal modes.
pub(in crate::tui) struct TerminalCoordinator {
    bridge: Arc<Bridge>,
    receiver: mpsc::Receiver<TerminalControl>,
    active: Option<OperationId>,
}

impl TerminalCoordinator {
    fn new() -> Self {
        // A worker holds the single slot until Resume is acknowledged, so at most
        // one handoff is queued. Waiting workers use a cancellable condition variable.
        let (sender, receiver) = mpsc::channel(2);
        Self {
            bridge: Arc::new(Bridge {
                sender,
                slot: Mutex::new(Slot::default()),
                available: Condvar::new(),
                closing: AtomicBool::new(false),
            }),
            receiver,
            active: None,
        }
    }

    pub(in crate::tui) fn install() -> Self {
        let coordinator = Self::new();
        *COORDINATOR.lock().unwrap() = Some(Arc::downgrade(&coordinator.bridge));
        coordinator
    }

    pub(in crate::tui) async fn recv(&mut self) -> Option<TerminalControl> {
        self.receiver.recv().await
    }

    pub(in crate::tui) fn owns_terminal(&self) -> bool {
        self.active.is_none()
    }

    #[allow(dead_code)] // shutdown uses this after the event-loop refactor
    pub(in crate::tui) fn is_idle(&self) -> bool {
        self.active.is_none() && self.bridge.slot.lock().unwrap().owner.is_none()
    }

    pub(in crate::tui) fn begin_shutdown(&mut self) {
        self.bridge.close();
    }

    /// App must stop reading terminal input before handling Suspend. A true return
    /// means Resume restored raw mode and the next frame needs a complete redraw.
    pub(in crate::tui) fn handle(&mut self, control: TerminalControl) -> Result<bool> {
        self.handle_with(
            control,
            || raw_mode::restore().map_err(Into::into),
            || raw_mode::setup().map_err(Into::into),
        )
    }

    fn handle_with(
        &mut self,
        control: TerminalControl,
        mut suspend: impl FnMut() -> Result<()>,
        mut resume: impl FnMut() -> Result<()>,
    ) -> Result<bool> {
        let TerminalControl {
            id,
            action,
            acknowledgement,
        } = control;
        match action {
            Action::Suspend => {
                let owner_matches = self.bridge.slot.lock().unwrap().owner == Some(id);
                if self.bridge.closing.load(Ordering::Acquire)
                    || !owner_matches
                    || self.active.is_some()
                {
                    if self.active != Some(id) {
                        self.bridge.release_owner(id);
                    }
                    let _ =
                        acknowledgement.try_send(Err("Terminal handoff cancelled or stale".into()));
                    return Ok(false);
                }
                if let Err(error) = suspend() {
                    self.bridge.release_owner(id);
                    let _ = acknowledgement.try_send(Err(error.to_string()));
                    return Err(error);
                }
                self.active = Some(id);
                self.bridge.slot.lock().unwrap().waiting_ack = None;
                if acknowledgement.try_send(Ok(())).is_err() {
                    // The requester cannot use the terminal if it never receives approval.
                    resume()?;
                    self.active = None;
                    self.bridge.release_owner(id);
                    return Ok(true);
                }
                Ok(false)
            }
            Action::Resume => {
                if self.active != Some(id) {
                    let _ = acknowledgement.try_send(Err("Stale terminal resume".into()));
                    return Ok(false);
                }
                let result = resume();
                self.active = None;
                // App may have no other wake source during shutdown. Release the
                // slot before acknowledging, rather than waiting for the worker
                // to wake up and make is_idle true without waking App again.
                self.bridge.release_owner(id);
                let _ = acknowledgement
                    .try_send(result.as_ref().map(|_| ()).map_err(ToString::to_string));
                result.map(|_| true)
            }
        }
    }
}

impl Drop for TerminalCoordinator {
    fn drop(&mut self) {
        self.bridge.close();
        let mut installed = COORDINATOR.lock().unwrap();
        if installed
            .as_ref()
            .and_then(Weak::upgrade)
            .is_some_and(|bridge| Arc::ptr_eq(&bridge, &self.bridge))
        {
            *installed = None;
        }
    }
}

/// A blocking worker's exclusive terminal lease. CLI calls receive an empty lease.
pub struct TerminalLease {
    bridge: Option<Arc<Bridge>>,
    id: OperationId,
}

impl TerminalLease {
    pub fn is_cancelled(&self) -> bool {
        self.bridge
            .as_ref()
            .is_some_and(|bridge| bridge.closing.load(Ordering::Acquire))
    }

    pub fn release(mut self) -> Result<()> {
        self.release_inner()
    }

    fn release_inner(&mut self) -> Result<()> {
        let Some(bridge) = self.bridge.take() else {
            return Ok(());
        };
        let (acknowledgement, reply) = sync_mpsc::sync_channel(1);
        let sent = bridge.sender.blocking_send(TerminalControl {
            id: self.id,
            action: Action::Resume,
            acknowledgement,
        });
        let result = match sent {
            Ok(()) => reply
                .recv()
                .map_err(|_| anyhow!("Terminal coordinator closed"))
                .and_then(|result| result.map_err(anyhow::Error::msg)),
            Err(_) => Err(anyhow!("Terminal coordinator closed")),
        };
        bridge.release_owner(self.id);
        result
    }
}

impl Drop for TerminalLease {
    fn drop(&mut self) {
        if let Err(error) = self.release_inner() {
            log::warn!("Terminal release: {error:#}");
        }
    }
}

fn acquire(bridge: Arc<Bridge>) -> Result<TerminalLease> {
    let (acknowledgement, reply) = sync_mpsc::sync_channel(1);
    let id = {
        let mut slot = bridge.slot.lock().unwrap();
        while slot.owner.is_some() && !bridge.closing.load(Ordering::Acquire) {
            slot = bridge.available.wait(slot).unwrap();
        }
        if bridge.closing.load(Ordering::Acquire) {
            bail!("Terminal handoff cancelled");
        }
        slot.next_id = slot
            .next_id
            .checked_add(1)
            .ok_or_else(|| anyhow!("Terminal operation IDs exhausted"))?;
        let id = OperationId(slot.next_id);
        slot.owner = Some(id);
        slot.waiting_ack = Some(acknowledgement.clone());
        id
    };
    if bridge
        .sender
        .blocking_send(TerminalControl {
            id,
            action: Action::Suspend,
            acknowledgement,
        })
        .is_err()
    {
        bridge.release_owner(id);
        bail!("Terminal coordinator closed");
    }
    let approved = reply
        .recv()
        .map_err(|_| anyhow!("Terminal coordinator closed"))
        .and_then(|value| value.map_err(anyhow::Error::msg));
    if let Err(error) = approved {
        bridge.release_owner(id);
        return Err(error);
    }
    let lease = TerminalLease {
        bridge: Some(bridge),
        id,
    };
    if lease.is_cancelled() {
        drop(lease);
        bail!("Terminal handoff cancelled");
    }
    Ok(lease)
}

/// Call only from a blocking worker, never from an App callback or async task.
pub fn terminal_lease() -> Result<TerminalLease> {
    let bridge = COORDINATOR.lock().unwrap().as_ref().and_then(Weak::upgrade);
    match bridge {
        Some(bridge) => acquire(bridge),
        None => Ok(TerminalLease {
            bridge: None,
            id: OperationId(0),
        }),
    }
}

#[allow(dead_code)] // command cancellation starts using this in the following refactor
pub(crate) fn shutdown_requested() -> bool {
    COORDINATOR
        .lock()
        .unwrap()
        .as_ref()
        .and_then(Weak::upgrade)
        .is_some_and(|bridge| bridge.closing.load(Ordering::Acquire))
}

pub fn setup() -> Result<()> {
    raw_mode::setup()?;
    raw_mode::set_panic_hook();
    Ok(())
}

pub fn teardown() {
    let _ = raw_mode::restore();
}

std::thread_local! {
    static HELD_LEASE: std::cell::RefCell<Option<TerminalLease>> =
        const { std::cell::RefCell::new(None) };
}

/// Lend the terminal to sudo or another foreground program.
/// Returns only after App has stopped the input reader.
pub fn hold(on: bool) -> Result<()> {
    if on {
        let lease = terminal_lease()?;
        HELD_LEASE.with(|slot| *slot.borrow_mut() = Some(lease));
        Ok(())
    } else if let Some(lease) = HELD_LEASE.with(|slot| slot.borrow_mut().take()) {
        lease.release()
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn resume_becomes_idle_before_requester_receives_acknowledgement() {
        let mut coordinator = TerminalCoordinator::new();
        let id = OperationId(1);
        coordinator.active = Some(id);
        coordinator.bridge.slot.lock().unwrap().owner = Some(id);
        let (acknowledgement, reply) = sync_mpsc::sync_channel(1);
        let returned = TerminalControl {
            id,
            action: Action::Resume,
            acknowledgement,
        };
        assert!(
            coordinator
                .handle_with(returned, || unreachable!(), || Ok(()))
                .unwrap()
        );
        // No worker receives the reply or sends another App wake in this test.
        assert!(coordinator.is_idle());
        assert!(reply.try_recv().unwrap().is_ok());
    }

    #[tokio::test]
    async fn cancelled_handoff_never_grants_terminal_and_releases_waiters() {
        let mut coordinator = TerminalCoordinator::new();
        let bridge = coordinator.bridge.clone();
        let worker = std::thread::spawn(move || acquire(bridge).is_err());
        let request = coordinator.recv().await.unwrap();
        coordinator.begin_shutdown();
        assert!(
            !coordinator
                .handle_with(
                    request,
                    || panic!("must not suspend"),
                    || panic!("must not resume")
                )
                .unwrap()
        );
        assert!(worker.join().unwrap());
        assert!(coordinator.is_idle());
    }

    #[tokio::test]
    async fn lease_returns_terminal_on_early_error_and_stale_resume_is_ignored() {
        let mut coordinator = TerminalCoordinator::new();
        let bridge = coordinator.bridge.clone();
        let worker = std::thread::spawn(move || {
            let _lease = acquire(bridge).unwrap();
            // Leaving scope is the same path used when spawning a command fails.
        });
        let request = coordinator.recv().await.unwrap();
        let id = request.id;
        assert!(
            !coordinator
                .handle_with(request, || Ok(()), || Ok(()))
                .unwrap()
        );
        assert!(!coordinator.owns_terminal());
        let returned = tokio::time::timeout(Duration::from_secs(1), coordinator.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(
            coordinator
                .handle_with(returned, || Ok(()), || Ok(()))
                .unwrap()
        );
        worker.join().unwrap();
        assert!(coordinator.is_idle());
        let (acknowledgement, _reply) = sync_mpsc::sync_channel(1);
        let stale = TerminalControl {
            id,
            action: Action::Resume,
            acknowledgement,
        };
        assert!(
            !coordinator
                .handle_with(
                    stale,
                    || panic!("unexpected suspend"),
                    || panic!("unexpected resume")
                )
                .unwrap()
        );
    }
}
