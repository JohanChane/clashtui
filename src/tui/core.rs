//! Core state, query demand, and actual task ownership.

use crate::config::{CONFIG, CoreType};
use crate::functions::restful::{
    self, config_struct::ClashConfig, connection::ConnInfo, proxies::ProxyCatalog,
};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Weak},
    task::{Context, Poll},
    time::{Duration, Instant},
};
use tokio::task::JoinHandle;

mod service;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ApiState {
    Unknown,
    Checking,
    Ready { version: String },
    Mismatch { actual: CoreType },
    Unavailable(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ServiceObservation {
    pub running: Option<bool>,
    pub description: String,
}
impl Default for ServiceObservation {
    fn default() -> Self {
        Self {
            running: None,
            description: "Not checked".into(),
        }
    }
}

pub(crate) struct CoreState {
    pub api: ApiState,
    pub services: [ServiceObservation; 2],
}
impl Default for CoreState {
    fn default() -> Self {
        Self {
            api: ApiState::Unknown,
            services: Default::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ApiResource {
    Status,
    Proxies,
    Connections,
    Settings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ServiceOperation {
    Start,
    Restart,
    Stop,
    StopAll,
    SwitchCore,
    #[cfg(windows)]
    Install,
    #[cfg(windows)]
    Uninstall,
}

#[derive(Default)]
pub(super) struct CoreEffects {
    pub invalidate_logs: bool,
    pub feedback: Vec<String>,
}

/// A disposable result and a separately owned actual job. Invalidation never drops the job.
pub(super) struct QuerySlot<T> {
    pub data: Option<Arc<T>>,
    pub error: Option<String>,
    inflight: Option<super::job::Job<T>>,
    operation: bool,
    report_success: bool,
    active: bool,
    requested: bool,
    due: Option<Instant>,
    period: Option<Duration>,
    revision: u64,
}
impl<T> Default for QuerySlot<T> {
    fn default() -> Self {
        Self {
            data: None,
            error: None,
            inflight: None,
            operation: false,
            report_success: true,
            active: false,
            requested: false,
            due: None,
            period: None,
            revision: 0,
        }
    }
}
struct Finished {
    feedback: Option<Result<(), String>>,
}
impl<T: Send + 'static> QuerySlot<T> {
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn busy(&self) -> bool {
        self.inflight.is_some()
    }
    pub fn invalidate(&mut self) {
        if let Some(job) = &mut self.inflight {
            job.accept_result = false;
        }
        self.data = None;
        self.error = None;
        // Active resources must resume without relying on a visible-page callback.
        self.requested = self.active;
        self.due = None;
        self.revision = self.revision.wrapping_add(1);
    }
    fn leave(&mut self) {
        self.active = false;
        self.invalidate();
    }
    fn enter(&mut self, period: Option<Duration>) {
        self.active = true;
        self.period = period;
        self.request();
    }
    fn request(&mut self) {
        if self.active {
            self.requested = true;
            self.due = None;
        }
    }
    fn start(
        &mut self,
        operation: bool,
        work: impl FnOnce() -> Result<T, String> + Send + 'static,
    ) -> bool {
        if self.busy() {
            return false;
        }
        self.requested = false;
        self.due = None;
        self.operation = operation;
        self.report_success = true;
        self.inflight = Some(super::job::Job::start(work));
        true
    }
    fn advance(&mut self, work: impl FnOnce() -> Result<T, String> + Send + 'static) {
        if self.active && self.requested && !self.busy() {
            self.start(false, work);
        }
    }
    fn poll_finished(&mut self, cx: &mut Context<'_>) -> Poll<Finished> {
        let Some(job) = &mut self.inflight else {
            return Poll::Pending;
        };
        let Poll::Ready(result) = job.poll(cx) else {
            return Poll::Pending;
        };
        let job = self.inflight.take().unwrap();
        let operation = self.operation;
        let report_success = self.report_success;
        self.operation = false;
        self.report_success = true;
        let result = result.unwrap_or_else(|error| Err(format!("Worker failed: {error}")));
        let feedback = (operation && (report_success || result.is_err()))
            .then(|| result.as_ref().map(|_| ()).map_err(Clone::clone));
        if job.accept_result {
            match result {
                Ok(value) => {
                    self.data = Some(Arc::new(value));
                    self.error = None;
                }
                Err(error) => {
                    self.data = None;
                    self.error = Some(error);
                }
            }
            self.revision = self.revision.wrapping_add(1);
            if self.active && self.error.is_none() && !self.requested {
                self.due = self.period.map(|period| Instant::now() + period);
            }
        }
        Poll::Ready(Finished { feedback })
    }
    fn deadline(&self) -> Option<Instant> {
        if self.active && !self.busy() {
            self.due
        } else {
            None
        }
    }
    fn make_due(&mut self, now: Instant) {
        if self.deadline().is_some_and(|due| due <= now) {
            self.request();
        }
    }
}

/// Checks need a pending demand and a disposable result, not a page refresh policy.
struct Check<T> {
    result: Option<Result<T, String>>,
    requested: bool,
    inflight: Option<super::job::Job<T>>,
}
impl<T> Default for Check<T> {
    fn default() -> Self {
        Self {
            result: None,
            requested: false,
            inflight: None,
        }
    }
}
impl<T: Send + 'static> Check<T> {
    fn busy(&self) -> bool {
        self.inflight.is_some()
    }
    fn accepts_result(&self) -> bool {
        self.inflight.as_ref().is_some_and(|job| job.accept_result)
    }
    fn invalidate(&mut self) {
        if let Some(job) = &mut self.inflight {
            job.accept_result = false;
        }
        self.result = None;
        self.requested = false;
    }
    fn start(&mut self, work: impl FnOnce() -> Result<T, String> + Send + 'static) -> bool {
        if self.busy() {
            return false;
        }
        self.requested = false;
        self.result = None;
        self.inflight = Some(super::job::Job::start(work));
        true
    }
    fn poll_finished(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        let Some(job) = &mut self.inflight else {
            return Poll::Pending;
        };
        let Poll::Ready(result) = job.poll(cx) else {
            return Poll::Pending;
        };
        if self.inflight.take().unwrap().accept_result {
            self.result =
                Some(result.unwrap_or_else(|error| Err(format!("Worker failed: {error}"))));
        }
        Poll::Ready(())
    }
}

#[derive(Default)]
pub(crate) struct ApiQueries {
    pub status: QuerySlot<ClashConfig>,
    pub proxies: QuerySlot<ProxyCatalog>,
    pub connections: QuerySlot<ConnInfo>,
    pub settings: QuerySlot<ClashConfig>,
}
impl ApiQueries {
    fn invalidate(&mut self) {
        self.status.invalidate();
        self.proxies.invalidate();
        self.connections.invalidate();
        self.settings.invalidate();
    }
}

#[derive(Default)]
struct CoreChecks {
    api: Check<restful::core_detect::CoreVersion>,
    services: [Check<ServiceObservation>; 2],
}

#[derive(Default)]
pub(crate) struct CoreBusiness {
    pub state: CoreState,
    pub queries: ApiQueries,
    checks: CoreChecks,
    service: Option<ServiceRun>,
    startup_until: Option<Instant>,
    check_retry_at: Option<Instant>,
    effects: CoreEffects,
    quitting: bool,
    api_lifetime: Option<Arc<()>>,
}

struct ServiceRun {
    target: CoreType,
    operation: ServiceOperation,
    task: Option<JoinHandle<Result<String, String>>>,
}

/// A deferred operation may use only the API availability period that admitted it.
pub(crate) struct ApiPermit(Weak<()>);
impl ApiPermit {
    pub fn is_valid(&self) -> bool {
        self.0.upgrade().is_some()
    }
}
fn index(core: CoreType) -> usize {
    match core {
        CoreType::Mihomo => 0,
        CoreType::Singbox => 1,
    }
}

impl CoreBusiness {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn take_effects(&mut self) -> CoreEffects {
        std::mem::take(&mut self.effects)
    }
    pub fn api_allowed(&self) -> bool {
        !self.quitting && self.service.is_none() && matches!(self.state.api, ApiState::Ready { .. })
    }
    pub fn api_permit(&self) -> Option<ApiPermit> {
        if !self.api_allowed() {
            return None;
        }
        self.api_lifetime
            .as_ref()
            .map(|life| ApiPermit(Arc::downgrade(life)))
    }
    pub fn diagnostic(&self) -> Option<String> {
        match &self.state.api {
            ApiState::Ready { .. } => None,
            ApiState::Unknown => Some("API not checked".into()),
            ApiState::Checking => Some(
                if self.service.is_some() {
                    "Core operation in progress..."
                } else if !self.service_ready_for_api(CONFIG.core_type()) {
                    "Checking core service..."
                } else {
                    "Checking API..."
                }
                .into(),
            ),
            ApiState::Mismatch { actual } => Some(format!(
                "API returned {actual} data, but {} is configured",
                CONFIG.core_type()
            )),
            ApiState::Unavailable(error) => Some(error.clone()),
        }
    }
    pub fn log_level(&self) -> Option<String> {
        self.queries
            .settings
            .data
            .as_ref()
            .or(self.queries.status.data.as_ref())
            .and_then(|data| data.log_level.as_ref().map(ToString::to_string))
    }
    fn set_api(&mut self, api: ApiState) {
        if self.state.api == api {
            return;
        }
        let was_allowed = matches!(self.state.api, ApiState::Ready { .. });
        let allowed = matches!(api, ApiState::Ready { .. });
        if allowed {
            self.api_lifetime.get_or_insert_with(|| Arc::new(()));
        } else {
            self.api_lifetime = None;
        }
        self.state.api = api;
        crate::config::set_core_mismatch(matches!(self.state.api, ApiState::Mismatch { .. }));
        if was_allowed && !allowed {
            self.queries.invalidate();
            self.effects.invalidate_logs = true;
        }
    }
    fn accept_service_observation(
        &mut self,
        target: CoreType,
        configured: CoreType,
        observed: ServiceObservation,
    ) {
        let i = index(target);
        self.state.services[i] = observed;
        if target == configured && self.state.services[i].running != Some(true) {
            // A check begun before this observation cannot restore API access.
            self.checks.api.invalidate();
            let error = if self.state.services[i].running == Some(false) {
                "Core service is stopped".into()
            } else {
                format!(
                    "Core service status is unknown: {}",
                    self.state.services[i].description
                )
            };
            self.set_api(ApiState::Unavailable(error));
            self.schedule_startup_retry();
        }
    }

    fn service_ready_for_api(&self, configured: CoreType) -> bool {
        let check = &self.checks.services[index(configured)];
        self.state.services[index(configured)].running == Some(true)
            && !check.busy()
            && !check.requested
    }

    fn request_service_check(&mut self, target: CoreType, configured: CoreType) {
        let check = &mut self.checks.services[index(target)];
        if check.accepts_result() {
            return;
        }
        check.requested = true;
        if target == configured {
            // Service confirmation is upstream. Retire an older API reply, but
            // retain its actual job and the demand to confirm API after Running.
            self.checks.api.invalidate();
            self.checks.api.requested = true;
            self.check_retry_at = None;
        }
    }

    fn request_api_confirmation(&mut self, configured: CoreType) {
        if self.checks.api.accepts_result() {
            return;
        }
        self.checks.api.requested = true;
        self.check_retry_at = None;
        if self.state.services[index(configured)].running != Some(true) {
            self.request_service_check(configured, configured);
        }
        if !matches!(self.state.api, ApiState::Ready { .. }) {
            self.set_api(ApiState::Checking);
        }
    }

    fn schedule_startup_retry(&mut self) -> bool {
        let now = Instant::now();
        if !self.quitting
            && let Some(until) = self.startup_until
            && until > now
        {
            self.check_retry_at = Some((now + Duration::from_millis(250)).min(until));
            true
        } else {
            self.startup_until = None;
            self.check_retry_at = None;
            false
        }
    }

    fn accept_api_observation(
        &mut self,
        configured: CoreType,
        observed: Result<restful::core_detect::CoreVersion, String>,
    ) {
        match observed {
            Ok(version) => {
                self.startup_until = None;
                self.check_retry_at = None;
                if version.core == configured {
                    self.set_api(ApiState::Ready {
                        version: version.version,
                    });
                } else {
                    self.set_api(ApiState::Mismatch {
                        actual: version.core,
                    });
                }
            }
            Err(error) => {
                if !self.schedule_startup_retry() {
                    self.set_api(ApiState::Unavailable(error));
                }
            }
        }
    }

    pub fn ensure_api(&mut self) {
        if self.quitting || self.service.is_some() {
            return;
        }
        self.request_api_confirmation(CONFIG.core_type());
    }
    pub fn check_services(&mut self) {
        if self.quitting || self.service.is_some() {
            return;
        }
        let configured = CONFIG.core_type();
        for target in [CoreType::Mihomo, CoreType::Singbox] {
            self.request_service_check(target, configured);
        }
    }
    pub fn enter(&mut self, resource: ApiResource) {
        if self.quitting {
            return;
        }
        match resource {
            ApiResource::Status => self.queries.status.enter(Some(Duration::from_secs(5))),
            ApiResource::Proxies => self.queries.proxies.enter(Some(Duration::from_secs(5))),
            ApiResource::Connections => {
                self.queries.connections.enter(Some(Duration::from_secs(1)))
            }
            ApiResource::Settings => self.queries.settings.enter(None),
        }
        self.ensure_api();
    }
    pub fn leave(&mut self, resource: ApiResource) {
        match resource {
            ApiResource::Status => self.queries.status.leave(),
            ApiResource::Proxies => self.queries.proxies.leave(),
            ApiResource::Connections => self.queries.connections.leave(),
            ApiResource::Settings => self.queries.settings.leave(),
        }
    }
    pub fn request_refresh(&mut self, resource: ApiResource) {
        match resource {
            ApiResource::Status => self.queries.status.request(),
            ApiResource::Proxies => self.queries.proxies.request(),
            ApiResource::Connections => self.queries.connections.request(),
            ApiResource::Settings => self.queries.settings.request(),
        }
    }
    pub fn advance(&mut self) {
        if self.quitting {
            return;
        }
        if let Some(service) = &mut self.service {
            if service.task.is_none() {
                let (target, operation) = (service.target, service.operation);
                service.task = Some(tokio::task::spawn_blocking(move || {
                    service::execute(target, operation)
                }));
            }
            return;
        }
        for target in [CoreType::Mihomo, CoreType::Singbox] {
            let check = &mut self.checks.services[index(target)];
            if check.requested && !check.busy() {
                check.start(move || service::observe(target));
            }
        }
        if self.checks.api.requested
            && !self.checks.api.busy()
            && self.service_ready_for_api(CONFIG.core_type())
        {
            self.checks
                .api
                .start(|| restful::core_detect::fetch_version().map_err(|e| e.to_string()));
        }
        if !self.api_allowed() {
            return;
        }
        macro_rules! start {
            ($slot:ident, $work:path) => {
                self.queries
                    .$slot
                    .advance(|| $work().map_err(|e| e.to_string()));
            };
        }
        start!(status, restful::config::fetch);
        start!(proxies, restful::proxies::fetch_proxies);
        start!(connections, restful::connection::get_connections);
        start!(settings, restful::config::fetch);
    }
    pub fn prepare_service(&mut self, target: CoreType, operation: ServiceOperation) -> bool {
        if self.quitting || self.service.is_some() {
            self.effects
                .feedback
                .push("A core operation is already running".into());
            return false;
        }
        self.set_api(ApiState::Checking);
        self.api_lifetime = None;
        self.queries.invalidate();
        self.checks.api.invalidate();
        for slot in &mut self.checks.services {
            slot.invalidate();
        }
        self.effects.invalidate_logs = true;
        self.check_retry_at = None;
        self.startup_until = None;
        self.service = Some(ServiceRun {
            target,
            operation,
            task: None,
        });
        true
    }
    pub fn run_proxy_operation(
        &mut self,
        work: impl FnOnce() -> Result<ProxyCatalog, String> + Send + 'static,
    ) -> bool {
        if !self.api_allowed() {
            return false;
        }
        let accepted = self.queries.proxies.start(true, work);
        if !accepted {
            self.effects
                .feedback
                .push("Proxy request is still running; try again when it finishes".into());
        }
        accepted
    }
    pub fn run_settings_operation(
        &mut self,
        work: impl FnOnce() -> Result<ClashConfig, String> + Send + 'static,
    ) {
        if !self.api_allowed() || self.queries.settings.data.is_none() {
            return;
        }
        if !self.queries.settings.start(true, work) {
            self.effects
                .feedback
                .push("Settings request is still running".into());
        }
    }
    pub fn run_connection_operation(
        &mut self,
        report_success: bool,
        work: impl FnOnce() -> Result<ConnInfo, String> + Send + 'static,
    ) {
        if !self.api_allowed() {
            return;
        }
        if !self.queries.connections.start(true, work) {
            self.effects
                .feedback
                .push("Connection request is still running".into());
        } else {
            self.queries.connections.report_success = report_success;
        }
    }
    pub fn poll_completion(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if let Some(task) = self
            .service
            .as_mut()
            .and_then(|service| service.task.as_mut())
            && let Poll::Ready(result) = Pin::new(task).poll(cx)
        {
            let service = self.service.take().unwrap();
            let result = result.map_err(|e| e.to_string()).and_then(|r| r);
            self.effects
                .feedback
                .push(result.unwrap_or_else(|e| format!("Core operation failed: {e}")));
            if !self.quitting {
                if matches!(
                    service.operation,
                    ServiceOperation::Start | ServiceOperation::Restart
                ) {
                    self.startup_until = Some(Instant::now() + Duration::from_secs(10));
                }
                self.check_services();
                self.ensure_api();
            }
            return Poll::Ready(());
        }
        if self.checks.api.poll_finished(cx).is_ready() {
            if let Some(observed) = self.checks.api.result.take() {
                self.accept_api_observation(CONFIG.core_type(), observed);
            }
            return Poll::Ready(());
        }
        for target in [CoreType::Mihomo, CoreType::Singbox] {
            let i = index(target);
            if self.checks.services[i].poll_finished(cx).is_ready() {
                if let Some(result) = self.checks.services[i].result.take() {
                    let observed = result.unwrap_or_else(|description| ServiceObservation {
                        running: None,
                        description,
                    });
                    self.accept_service_observation(target, CONFIG.core_type(), observed);
                }
                return Poll::Ready(());
            }
        }
        macro_rules! complete {
            ($slot:ident) => {
                if let Poll::Ready(done) = self.queries.$slot.poll_finished(cx) {
                    if let Some(result) = done.feedback {
                        self.effects.feedback.push(
                            result
                                .map(|_| "Operation completed".into())
                                .unwrap_or_else(|e| format!("Operation failed: {e}")),
                        );
                    }
                    return Poll::Ready(());
                }
            };
        }
        complete!(status);
        complete!(proxies);
        complete!(connections);
        complete!(settings);
        Poll::Pending
    }
    pub fn next_deadline(&self) -> Option<Instant> {
        if self.quitting {
            return None;
        }
        let mut deadline = self.check_retry_at;
        if self.api_allowed() {
            for due in [
                self.queries.status.deadline(),
                self.queries.proxies.deadline(),
                self.queries.connections.deadline(),
                self.queries.settings.deadline(),
            ]
            .into_iter()
            .flatten()
            {
                deadline = Some(deadline.map_or(due, |old| old.min(due)));
            }
        }
        deadline
    }
    pub fn run_due(&mut self) {
        let now = Instant::now();
        if self.check_retry_at.is_some_and(|due| due <= now) {
            self.check_retry_at = None;
            if self.startup_until.is_some_and(|until| until > now) {
                self.ensure_api();
            } else {
                self.startup_until = None;
                if matches!(self.state.api, ApiState::Checking) {
                    self.set_api(ApiState::Unavailable(
                        "Timed out waiting for core API".into(),
                    ));
                }
            }
        }
        if self.api_allowed() {
            self.queries.status.make_due(now);
            self.queries.proxies.make_due(now);
            self.queries.connections.make_due(now);
            self.queries.settings.make_due(now);
        }
    }
    pub fn has_tasks(&self) -> bool {
        self.service.is_some()
            || self.checks.api.busy()
            || self.checks.services.iter().any(Check::busy)
            || self.queries.status.busy()
            || self.queries.proxies.busy()
            || self.queries.connections.busy()
            || self.queries.settings.busy()
    }
    pub fn begin_shutdown(&mut self) {
        self.quitting = true;
        if self.service.as_ref().is_some_and(|run| run.task.is_none()) {
            self.service = None;
        }
        self.api_lifetime = None;
        self.check_retry_at = None;
        self.startup_until = None;
        self.queries.status.leave();
        self.queries.proxies.leave();
        self.queries.connections.leave();
        self.queries.settings.leave();
        self.checks.api.invalidate();
        for check in &mut self.checks.services {
            check.invalidate();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::poll_fn;
    use tokio::sync::oneshot;

    fn service_observation(running: Option<bool>) -> ServiceObservation {
        ServiceObservation {
            running,
            description: match running {
                Some(true) => "active",
                Some(false) => "inactive",
                None => "service probe failed",
            }
            .into(),
        }
    }

    async fn complete_service_check(
        business: &mut CoreBusiness,
        target: CoreType,
        configured: CoreType,
        observed: ServiceObservation,
    ) {
        let check = &mut business.checks.services[index(target)];
        assert!(check.start(move || Ok(observed)));
        poll_fn(|cx| check.poll_finished(cx)).await;
        let observed = check.result.take().unwrap().unwrap();
        business.accept_service_observation(target, configured, observed.clone());
    }

    #[tokio::test]
    async fn api_confirmation_waits_for_service_and_preserves_its_observation() {
        let mut business = CoreBusiness::default();
        business.request_api_confirmation(CoreType::Mihomo);
        assert!(business.checks.services[0].requested);
        assert!(!business.checks.services[1].requested);
        assert!(business.checks.api.requested);
        assert!(!business.service_ready_for_api(CoreType::Mihomo));

        let running = service_observation(Some(true));
        complete_service_check(
            &mut business,
            CoreType::Mihomo,
            CoreType::Mihomo,
            running.clone(),
        )
        .await;
        assert!(business.service_ready_for_api(CoreType::Mihomo));
        assert!(business.checks.api.requested);
        assert!(!business.api_allowed(), "Running is not API Ready");

        business.accept_api_observation(
            CoreType::Mihomo,
            Ok(restful::core_detect::CoreVersion {
                core: CoreType::Mihomo,
                version: "current".into(),
            }),
        );
        assert!(business.api_allowed());
        assert_eq!(business.state.services[0], running);
        assert!(!business.checks.services[0].requested);
        assert!(business.next_deadline().is_none());
    }

    #[tokio::test]
    async fn new_service_check_retires_api_reply_and_retains_confirmation_demand() {
        let mut business = CoreBusiness::default();
        let running = service_observation(Some(true));
        business.state.services[0] = running.clone();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        business.checks.api.start(move || {
            release_rx.recv().unwrap();
            Ok(restful::core_detect::CoreVersion {
                core: CoreType::Mihomo,
                version: "old".into(),
            })
        });
        business.request_service_check(CoreType::Mihomo, CoreType::Mihomo);
        assert!(!business.checks.api.accepts_result());
        assert!(business.checks.api.busy());
        assert!(business.checks.api.requested);
        assert!(!business.service_ready_for_api(CoreType::Mihomo));

        release_tx.send(()).unwrap();
        poll_fn(|cx| business.checks.api.poll_finished(cx)).await;
        assert!(business.checks.api.result.is_none());
        assert!(business.checks.api.requested);
        assert!(!business.service_ready_for_api(CoreType::Mihomo));

        // An unchanged Running observation must still advance the confirmation.
        complete_service_check(&mut business, CoreType::Mihomo, CoreType::Mihomo, running).await;
        assert!(business.service_ready_for_api(CoreType::Mihomo));
        assert!(business.checks.api.requested);
    }

    #[tokio::test]
    async fn stopped_or_unknown_service_blocks_api_without_an_automatic_retry_loop() {
        for running in [Some(false), None] {
            let mut business = CoreBusiness::default();
            let observed = service_observation(running);
            business.state.services[0] = observed.clone();
            business.request_api_confirmation(CoreType::Mihomo);
            assert!(business.checks.services[0].requested);
            complete_service_check(&mut business, CoreType::Mihomo, CoreType::Mihomo, observed)
                .await;
            assert_eq!(business.state.services[0].running, running);
            assert!(!business.checks.api.requested);
            assert!(!business.api_allowed());
            assert!(business.next_deadline().is_none());
            let ApiState::Unavailable(error) = &business.state.api else {
                panic!("API must stay closed until service can be confirmed");
            };
            assert_eq!(error.contains("unknown"), running.is_none());

            // An explicit refresh can discover a later external service start.
            business.request_api_confirmation(CoreType::Mihomo);
            assert!(business.checks.services[0].requested);
            assert!(business.checks.api.requested);
        }
    }

    #[test]
    fn known_running_service_is_reused_but_pending_checks_block_api_start() {
        let mut business = CoreBusiness::default();
        business.state.services[0] = service_observation(Some(true));
        business.request_api_confirmation(CoreType::Mihomo);
        assert!(business.checks.api.requested);
        assert!(!business.checks.services[0].requested);
        assert!(business.service_ready_for_api(CoreType::Mihomo));
        business.request_service_check(CoreType::Mihomo, CoreType::Mihomo);
        assert!(!business.service_ready_for_api(CoreType::Mihomo));
    }

    #[tokio::test]
    async fn other_core_service_check_does_not_retire_current_api_confirmation() {
        let mut business = CoreBusiness::default();
        business.state.services[0] = service_observation(Some(true));
        business.checks.api.start(|| {
            Ok(restful::core_detect::CoreVersion {
                core: CoreType::Mihomo,
                version: "current".into(),
            })
        });
        business.request_service_check(CoreType::Singbox, CoreType::Mihomo);
        assert!(business.checks.api.accepts_result());
        assert!(business.service_ready_for_api(CoreType::Mihomo));
        complete_service_check(
            &mut business,
            CoreType::Singbox,
            CoreType::Mihomo,
            service_observation(Some(false)),
        )
        .await;
        assert!(business.checks.api.accepts_result());
        poll_fn(|cx| business.checks.api.poll_finished(cx)).await;
        assert!(business.checks.api.result.is_some());
        assert!(business.next_deadline().is_none());
    }

    #[test]
    fn startup_retries_the_unconfirmed_stage_and_stops_after_its_deadline() {
        let mut business = CoreBusiness {
            startup_until: Some(Instant::now() + Duration::from_secs(10)),
            ..Default::default()
        };
        business.accept_service_observation(
            CoreType::Mihomo,
            CoreType::Mihomo,
            service_observation(Some(false)),
        );
        assert!(business.check_retry_at.is_some());
        business.request_api_confirmation(CoreType::Mihomo);
        assert!(business.checks.services[0].requested);
        assert!(!business.service_ready_for_api(CoreType::Mihomo));

        business.checks.services[0].requested = false;
        business.state.services[0] = service_observation(Some(true));
        business.accept_api_observation(CoreType::Mihomo, Err("API not ready".into()));
        assert_eq!(business.state.services[0].running, Some(true));
        assert!(business.check_retry_at.is_some());
        business.request_api_confirmation(CoreType::Mihomo);
        assert!(!business.checks.services[0].requested);
        assert!(business.checks.api.requested);

        business.startup_until = Some(Instant::now() - Duration::from_secs(1));
        business.accept_api_observation(CoreType::Mihomo, Err("API still unavailable".into()));
        assert!(business.check_retry_at.is_none());
        assert!(business.startup_until.is_none());
        assert_eq!(
            business.state.api,
            ApiState::Unavailable("API still unavailable".into())
        );
        assert_eq!(business.state.services[0].running, Some(true));
    }

    #[test]
    fn expired_startup_timer_finishes_without_starting_another_check() {
        let mut business = CoreBusiness::default();
        business.state.services[0] = service_observation(Some(true));
        business.set_api(ApiState::Checking);
        business.startup_until = Some(Instant::now() - Duration::from_secs(1));
        business.check_retry_at = business.startup_until;
        business.run_due();
        assert!(!business.has_tasks());
        assert!(business.next_deadline().is_none());
        assert!(business.startup_until.is_none());
        assert_eq!(
            business.state.api,
            ApiState::Unavailable("Timed out waiting for core API".into())
        );
    }

    #[tokio::test]
    async fn invalidating_a_reply_keeps_the_actual_slot_until_completion() {
        let mut slot = QuerySlot::<u32>::default();
        slot.enter(Some(Duration::from_secs(1)));
        let (started_tx, started_rx) = oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        assert!(slot.start(false, move || {
            let _ = started_tx.send(());
            release_rx.recv().unwrap();
            Ok(1)
        }));
        started_rx.await.unwrap();
        slot.invalidate();
        assert!(slot.busy());
        assert!(!slot.start(false, || panic!("a second actual request must not start")));
        release_tx.send(()).unwrap();
        poll_fn(|cx| slot.poll_finished(cx)).await;
        assert!(
            slot.data.is_none(),
            "invalidated result cannot restore the cache"
        );
        assert!(
            slot.requested,
            "completion must retain the blocked refresh demand"
        );
        assert!(slot.start(false, || Ok(2)));
        poll_fn(|cx| slot.poll_finished(cx)).await;
        assert_eq!(**slot.data.as_ref().unwrap(), 2);
    }

    #[tokio::test]
    async fn leaving_discards_query_data_but_keeps_operation_feedback() {
        let mut slot = QuerySlot::<u32>::default();
        slot.enter(None);
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        slot.start(true, move || {
            release_rx.recv().unwrap();
            Err("write failed".to_owned())
        });
        slot.leave();
        release_tx.send(()).unwrap();
        let done = poll_fn(|cx| slot.poll_finished(cx)).await;
        assert_eq!(done.feedback, Some(Err("write failed".into())));
        assert!(slot.data.is_none());
        assert!(slot.error.is_none());
        assert!(!slot.requested);
        assert!(slot.deadline().is_none());
    }

    #[tokio::test]
    async fn successful_visible_queries_schedule_data_refresh_but_errors_do_not_loop() {
        let mut slot = QuerySlot::<u32>::default();
        slot.enter(Some(Duration::from_secs(1)));
        slot.start(false, || Ok(10));
        poll_fn(|cx| slot.poll_finished(cx)).await;
        assert!(slot.deadline().is_some());
        slot.request();
        slot.start(false, || Err("resource unavailable".into()));
        poll_fn(|cx| slot.poll_finished(cx)).await;
        assert!(slot.data.is_none());
        assert_eq!(slot.error.as_deref(), Some("resource unavailable"));
        assert!(slot.deadline().is_none());
        assert!(!slot.requested);
    }

    #[tokio::test]
    async fn shutdown_reaps_actual_work_without_accepting_its_old_reply() {
        let mut business = CoreBusiness::default();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        business
            .queries
            .connections
            .enter(Some(Duration::from_secs(1)));
        business.queries.connections.start(false, move || {
            release_rx.recv().unwrap();
            Ok(ConnInfo::default())
        });
        business.begin_shutdown();
        assert!(business.has_tasks());
        assert!(business.next_deadline().is_none());
        release_tx.send(()).unwrap();
        poll_fn(|cx| business.poll_completion(cx)).await;
        assert!(!business.has_tasks());
        assert!(business.queries.connections.data.is_none());
    }

    #[test]
    fn api_failure_does_not_fabricate_a_stopped_service() {
        let mut business = CoreBusiness::default();
        business.state.services[0] = ServiceObservation {
            running: Some(true),
            description: "active".into(),
        };
        business.set_api(ApiState::Unavailable("connection refused".into()));
        assert_eq!(business.state.services[0].running, Some(true));
        assert!(!business.api_allowed());
    }

    #[tokio::test]
    async fn stopped_observation_discards_an_older_api_reply_before_notification() {
        let mut business = CoreBusiness::default();
        business.set_api(ApiState::Ready {
            version: "old".into(),
        });
        business.queries.connections.data = Some(Arc::new(ConnInfo::default()));
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        business.checks.api.start(move || {
            release_rx.recv().unwrap();
            Ok(restful::core_detect::CoreVersion {
                core: CoreType::Mihomo,
                version: "stale".into(),
            })
        });
        business.accept_service_observation(
            CoreType::Mihomo,
            CoreType::Mihomo,
            ServiceObservation {
                running: Some(false),
                description: "inactive".into(),
            },
        );
        assert!(!business.api_allowed());
        assert!(business.queries.connections.data.is_none());
        assert!(!business.checks.api.accepts_result());
        assert!(business.checks.api.busy());
        assert!(business.take_effects().invalidate_logs);
        release_tx.send(()).unwrap();
        poll_fn(|cx| business.poll_completion(cx)).await;
        assert!(matches!(business.state.api, ApiState::Unavailable(_)));
        assert!(!business.has_tasks());
    }

    #[tokio::test]
    async fn closed_api_and_shutdown_refuse_new_operation_work() {
        let mut business = CoreBusiness::default();
        business.run_proxy_operation(|| panic!("unknown API must reject proxy operations"));
        business.run_settings_operation(|| panic!("unknown API must reject config writes"));
        business
            .run_connection_operation(true, || panic!("unknown API must reject connection writes"));
        assert!(!business.has_tasks());

        business.set_api(ApiState::Ready {
            version: "v1".into(),
        });
        business.begin_shutdown();
        business.run_proxy_operation(|| panic!("shutdown must reject proxy operations"));
        business.run_settings_operation(|| panic!("shutdown must reject config writes"));
        business
            .run_connection_operation(true, || panic!("shutdown must reject connection writes"));
        business.enter(ApiResource::Status);
        business.check_services();
        assert!(!business.prepare_service(CoreType::Mihomo, ServiceOperation::Stop));
        assert!(!business.has_tasks());
        assert!(business.next_deadline().is_none());
    }

    #[test]
    fn a_deferred_operation_cannot_reuse_permission_after_api_recovery() {
        let mut business = CoreBusiness::default();
        business.set_api(ApiState::Ready {
            version: "first".into(),
        });
        let old = business.api_permit().unwrap();
        assert!(old.is_valid());
        business.set_api(ApiState::Checking);
        assert!(!old.is_valid());
        assert!(business.api_permit().is_none());
        business.set_api(ApiState::Ready {
            version: "second".into(),
        });
        let current = business.api_permit().unwrap();
        assert!(current.is_valid());
        assert!(
            !old.is_valid(),
            "a stopped availability period must stay closed"
        );
        business.begin_shutdown();
        assert!(!current.is_valid());
        assert!(business.api_permit().is_none());
    }
    #[tokio::test]
    async fn invalidation_retains_active_demand_and_reenter_does_not_accept_old_work() {
        let mut slot = QuerySlot::<u32>::default();
        slot.enter(None);
        let (release, wait) = std::sync::mpsc::channel();
        slot.advance(move || {
            wait.recv().unwrap();
            Ok(1)
        });
        slot.leave();
        slot.enter(None);
        slot.advance(|| panic!("the old job still occupies the slot"));
        release.send(()).unwrap();
        poll_fn(|cx| slot.poll_finished(cx)).await;
        assert!(slot.data.is_none());
        assert!(slot.requested);
        slot.advance(|| Ok(2));
        poll_fn(|cx| slot.poll_finished(cx)).await;
        assert_eq!(slot.data.as_deref(), Some(&2));
        assert!(!slot.requested);
    }

    #[test]
    fn api_loss_preserves_only_visible_resource_demand_and_retires_permits() {
        let mut core = CoreBusiness::default();
        core.set_api(ApiState::Ready {
            version: "first".into(),
        });
        let permit = core.api_permit().unwrap();
        core.queries.connections.enter(None);
        core.queries.connections.requested = false;
        core.set_api(ApiState::Unavailable("offline".into()));
        assert!(core.queries.connections.requested);
        assert!(!core.queries.settings.requested);
        core.set_api(ApiState::Ready {
            version: "second".into(),
        });
        assert!(core.queries.connections.requested);
        assert!(!permit.is_valid());
        assert!(core.api_permit().unwrap().is_valid());
    }

    #[test]
    fn shutdown_drops_prepared_service_and_rejection_does_not_invalidate_logs() {
        let mut core = CoreBusiness::default();
        assert!(core.prepare_service(CoreType::Mihomo, ServiceOperation::Stop));
        assert!(core.service.as_ref().unwrap().task.is_none());
        assert!(core.take_effects().invalidate_logs);
        assert!(!core.prepare_service(CoreType::Mihomo, ServiceOperation::Start));
        let effects = core.take_effects();
        assert!(!effects.invalidate_logs);
        assert_eq!(effects.feedback.len(), 1);
        core.begin_shutdown();
        core.advance();
        assert!(!core.has_tasks());
    }

    #[tokio::test]
    async fn a_panicked_operation_releases_its_slot_and_keeps_feedback_after_leave() {
        let mut slot = QuerySlot::<()>::default();
        slot.enter(None);
        slot.start(true, || panic!("worker crashed"));
        slot.leave();
        let done = poll_fn(|cx| slot.poll_finished(cx)).await;
        assert!(
            done.feedback
                .unwrap()
                .unwrap_err()
                .contains("Worker failed")
        );
        assert!(!slot.busy());
        assert!(slot.data.is_none());
        assert!(slot.error.is_none());
    }

    fn ready_core() -> CoreBusiness {
        let mut business = CoreBusiness::default();
        business.set_api(ApiState::Ready {
            version: "v1".into(),
        });
        business
    }

    #[tokio::test]
    async fn silent_connection_success_has_no_feedback() {
        let mut business = ready_core();
        business.run_connection_operation(false, || Ok(ConnInfo::default()));
        assert!(business.queries.connections.busy());
        let done = poll_fn(|cx| business.queries.connections.poll_finished(cx)).await;
        assert_eq!(done.feedback, None);
    }

    #[tokio::test]
    async fn silent_connection_failure_still_reports_the_error() {
        let mut business = ready_core();
        business.run_connection_operation(false, || Err("write failed".into()));
        assert!(business.queries.connections.busy());
        let done = poll_fn(|cx| business.queries.connections.poll_finished(cx)).await;
        assert_eq!(done.feedback, Some(Err("write failed".into())));
    }

    #[tokio::test]
    async fn a_following_operation_does_not_inherit_silent_success() {
        let mut business = ready_core();
        business.run_connection_operation(false, || Ok(ConnInfo::default()));
        let done = poll_fn(|cx| business.queries.connections.poll_finished(cx)).await;
        assert_eq!(done.feedback, None);
        assert!(
            business
                .queries
                .connections
                .start(true, || Ok(ConnInfo::default()))
        );
        let done = poll_fn(|cx| business.queries.connections.poll_finished(cx)).await;
        assert_eq!(done.feedback, Some(Ok(())));
    }
}
