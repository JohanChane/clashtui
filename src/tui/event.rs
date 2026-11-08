//! All events a page can receive. Match a variant in `on_event` to subscribe to it.

use crate::config::{CONFIG, CoreType};

mod bus;
pub(in crate::tui) use bus::{Inbox, emit};

/// All events a page can receive. Match a variant in on_event to subscribe to it.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    // Lifecycle: Hub or Host delivers to a specific tab; these events cannot be emitted.
    /// Delivered once to each tab at startup, before any Shown event.
    Init,
    /// The tab becomes visible. Both sides of Split receive this event.
    Shown,
    /// The tab becomes hidden and stops receiving Tick and broadcasts.
    Hidden,
    /// Delivered at the Page::TICK interval while visible.
    Tick,

    // Broadcasts: emit from anywhere; delivered to CoreMonitor and the visible tab.
    Core(CoreEvent),
    File(FileEvent),
}

/// Changes to the core (mihomo / sing-box).
#[derive(Clone, Debug, PartialEq)]
pub enum CoreEvent {
    /// Core availability changed. Only CoreMonitor emits this, and only when the status changes.
    StatusChanged(CoreStatus),
    /// clashtui completed a service command (start, stop, restart, install, or uninstall), successfully or not.
    /// This is not a status: CoreMonitor rechecks CoreStatus and CoreSrvCtl refreshes its service display.
    /// Other pages do not handle it.
    ServiceCommandDone,
    /// The configured core type changed. App exits on receiving this event.
    Switched(CoreType),
    /// Runtime configuration was reloaded (profile selection or current-profile update) or changed (Settings).
    ConfigChanged,
    /// All connections were closed outside the Connections page.
    ConnectionsClosed,
}

/// Changes to files managed by clashtui.
#[derive(Clone, Debug, PartialEq)]
pub enum FileEvent {
    /// Profile list or metadata changed: import, delete, update, select, toggle options, or generate from a template.
    ProfilesChanged,
}

/// Current state of the configured core, combining service and API status:
/// a stopped service means the API is unreachable; a running service does not guarantee API access.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum CoreStatus {
    /// No confirmed result yet: at startup or awaiting confirmation after a service command.
    #[default]
    Unknown,
    /// The configured core service is stopped, so its API is unreachable.
    Stopped,
    /// The service is running or its state is unknown, but the API is unreachable. Display reason to the user.
    Unreachable { reason: String },
    /// The API returned data from a different core.
    Mismatch { running: CoreType },
    /// The API is available and belongs to the configured core.
    Ready { version: String },
}

impl CoreStatus {
    /// Ok permits API requests; Err contains the reason the page should display.
    pub fn ready(&self) -> Result<(), String> {
        match self {
            Self::Ready { .. } => Ok(()),
            Self::Unknown => Err("Detecting core...".into()),
            Self::Stopped => Err("Core service is not running".into()),
            Self::Unreachable { reason } => Err(reason.clone()),
            Self::Mismatch { running } => Err(format!(
                "API returned {running} data, but {} is configured",
                CONFIG.core_type()
            )),
        }
    }
}

/// Events that can be broadcast. Lifecycle events do not implement this, so emit(Event::Shown) cannot compile.
pub trait Broadcast: Into<Event> + Send + 'static {}
impl Broadcast for CoreEvent {}
impl Broadcast for FileEvent {}
impl From<CoreEvent> for Event {
    fn from(e: CoreEvent) -> Self {
        Self::Core(e)
    }
}
impl From<FileEvent> for Event {
    fn from(e: FileEvent) -> Self {
        Self::File(e)
    }
}
