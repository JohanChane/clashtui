//! The commit boundary for shared state. Views cannot start work.
use super::{
    core::{ApiPermit, ApiQueries, ApiResource, CoreBusiness, CoreState, ServiceOperation},
    tab::logs::{LogCollector, Snapshot},
};
use crate::{config::CoreType, functions::restful};
use std::{
    sync::Arc,
    task::{Context, Poll},
    time::Instant,
};
use tokio::sync::Notify;

#[derive(Clone, Copy)]
pub(in crate::tui) struct CoreView<'a> {
    pub state: &'a CoreState,
    pub queries: &'a ApiQueries,
    owner: &'a CoreBusiness,
}
impl CoreView<'_> {
    pub fn api_allowed(self) -> bool {
        self.owner.api_allowed()
    }
    pub fn diagnostic(self) -> Option<String> {
        self.owner.diagnostic()
    }
}
#[derive(Clone, Copy)]
pub(in crate::tui) struct BusinessView<'a> {
    pub core: CoreView<'a>,
    logs: &'a LogCollector,
}
impl BusinessView<'_> {
    /// Copy the bounded buffer only at the explicit Logs display boundary.
    pub fn log_snapshot(self) -> Snapshot {
        self.logs.snapshot()
    }
}

pub(in crate::tui) enum SettingsMaintenance {
    FlushFakeIp,
    FlushDns,
    UpdateGeo,
}

pub(in crate::tui) enum Completion {
    Core,
    Logs,
}

pub(in crate::tui) struct Business {
    core: CoreBusiness,
    logs: LogCollector,
    feedback: Vec<String>,
}
impl Default for Business {
    fn default() -> Self {
        Self {
            core: CoreBusiness::new(),
            logs: LogCollector::new(),
            feedback: Vec::new(),
        }
    }
}
impl Business {
    pub fn view(&self) -> BusinessView<'_> {
        BusinessView {
            core: CoreView {
                state: &self.core.state,
                queries: &self.core.queries,
                owner: &self.core,
            },
            logs: &self.logs,
        }
    }
    /// A finite commit: invalidate dependent state before advancing actual work.
    fn settle(&mut self) {
        let effects = self.core.take_effects();
        if effects.invalidate_logs {
            self.logs.invalidate_connection();
        }
        self.feedback.extend(effects.feedback);
        self.core.advance();
        let level = self.core.log_level();
        self.logs.set_core_state(
            self.core.api_allowed(),
            self.core.diagnostic(),
            level.as_deref(),
        );
    }
    pub fn take_feedback(&mut self) -> Vec<String> {
        std::mem::take(&mut self.feedback)
    }
    pub fn api_permit(&self) -> Option<ApiPermit> {
        self.core.api_permit()
    }
    pub fn enter(&mut self, resource: ApiResource) {
        self.core.enter(resource);
        self.settle();
    }
    pub fn leave(&mut self, resource: ApiResource) {
        self.core.leave(resource);
        self.settle();
    }
    pub fn confirm_api(&mut self) {
        self.core.ensure_api();
        self.settle();
    }
    pub fn refresh(&mut self, resource: ApiResource) {
        self.core.ensure_api();
        self.core.request_refresh(resource);
        self.settle();
    }
    pub fn check_services(&mut self) {
        self.core.check_services();
        self.settle();
    }
    pub fn start_service(&mut self, target: CoreType, operation: ServiceOperation) -> bool {
        let accepted = self.core.prepare_service(target, operation);
        self.settle();
        accepted
    }
    pub fn next_deadline(&self) -> Option<Instant> {
        self.core.next_deadline()
    }
    pub fn run_due(&mut self) {
        self.core.run_due();
        self.settle();
    }
    pub fn poll_completion(&mut self, cx: &mut Context<'_>) -> Poll<Completion> {
        if self.core.poll_completion(cx).is_ready() {
            self.settle();
            return Poll::Ready(Completion::Core);
        }
        self.logs.poll_finished(cx).map(|()| Completion::Logs)
    }
    pub fn has_work(&self) -> bool {
        self.core.has_tasks() || self.logs.has_worker()
    }
    pub fn has_tasks(&self) -> bool {
        self.core.has_tasks()
    }
    pub fn log_notification(&self) -> Arc<Notify> {
        self.logs.notification()
    }
    pub fn has_log_worker(&self) -> bool {
        self.logs.has_worker()
    }
    pub fn pause_logs(&mut self, paused: bool) {
        self.logs.set_paused(paused);
        self.settle();
    }
    pub fn clear_logs(&mut self) {
        self.logs.clear();
        self.settle();
    }
    pub fn set_log_level(&mut self, level: &str) {
        self.logs.set_level(level);
        self.settle();
    }
    pub fn begin_shutdown(&mut self) {
        self.core.begin_shutdown();
        self.logs.begin_shutdown();
        self.settle();
    }
    #[cfg(windows)]
    pub fn toggle_system_proxy(&mut self) -> tokio::task::JoinHandle<anyhow::Result<bool>> {
        let permit = self.api_permit();
        if permit.is_none() {
            self.confirm_api();
        }
        let work = tokio::task::spawn_blocking(move || {
            use crate::functions::command;
            let enabled = command::get_system_proxy_state()?;
            if enabled {
                command::disable_system_proxy()?;
            } else {
                let allowed = || permit.as_ref().is_some_and(|permit| permit.is_valid());
                anyhow::ensure!(
                    allowed(),
                    "API confirmation requested; retry once the core API is available"
                );
                let config = restful::config::fetch()?;
                let port = config.mixed_port.or(config.port).ok_or_else(|| {
                    anyhow::anyhow!("No mixed-port or port returned by the core API")
                })?;
                anyhow::ensure!(
                    allowed(),
                    "Core API permission expired before enabling system proxy"
                );
                command::enable_system_proxy(port)?;
            }
            Ok(!enabled)
        });
        self.settle();
        work
    }
    pub fn patch_settings(&mut self, payload: String) {
        self.core.run_settings_operation(move || {
            restful::config::patch(payload).map_err(|e| e.to_string())?;
            restful::config::fetch().map_err(|e| e.to_string())
        });
        self.settle();
    }
    pub fn maintain_settings(&mut self, operation: SettingsMaintenance) {
        self.core.run_settings_operation(move || {
            match operation {
                SettingsMaintenance::FlushFakeIp => restful::cache::flush_fakeip(),
                SettingsMaintenance::FlushDns => restful::cache::flush_dns(),
                SettingsMaintenance::UpdateGeo => restful::geo::upgrade_geo(),
            }
            .map_err(|e| e.to_string())?;
            restful::config::fetch().map_err(|e| e.to_string())
        });
        self.settle();
    }
    /// None closes all; Some closes only the IDs selected by the current filter.
    pub fn close_connections(&mut self, ids: Option<Vec<String>>) {
        self.submit_connection_close(ids, true);
    }
    pub fn close_all_connections(&mut self) {
        self.submit_connection_close(None, false);
    }
    fn submit_connection_close(&mut self, ids: Option<Vec<String>>, report_success: bool) {
        if let Some(permit) = self.core.api_permit() {
            self.core.run_connection_operation(report_success, move || {
                let check = || {
                    if permit.is_valid() {
                        Ok(())
                    } else {
                        Err(
                            "Core API permission expired; remaining connections were not closed"
                                .to_owned(),
                        )
                    }
                };
                check()?;
                if let Some(ids) = ids {
                    for id in ids {
                        check()?;
                        restful::connection::terminate_connection(Some(id))
                            .map_err(|e| e.to_string())?;
                    }
                } else {
                    restful::connection::terminate_all_connections().map_err(|e| e.to_string())?;
                }
                if !permit.is_valid() {
                    return Err("Connections closed; API permission expired before refresh".into());
                }
                restful::connection::get_connections().map_err(|e| e.to_string())
            });
        }
        self.settle();
    }
}

use crate::functions::restful::proxies;
use crate::tui::popmsg::prelude::Confirm;
impl Business {
    pub fn select_inline(&mut self, group: String, node: String) -> bool {
        if !self.core.api_allowed() {
            return false;
        }
        let accepted = self.core.run_proxy_operation(move || {
            proxies::select_proxy(&group, &node).map_err(|e| e.to_string())?;
            proxies::fetch_proxies().map_err(|e| e.to_string())
        });
        self.settle();
        accepted
    }
    pub fn test_delay(&mut self, name: String, group: bool) -> bool {
        if !self.core.api_allowed() {
            return false;
        }
        let timeout = crate::config::CONFIG.cfg_file.timeout.unwrap_or(5) * 1000;
        let Some(before) = self.core.queries.proxies.data.clone() else {
            return false;
        };
        let accepted = if group {
            let url = before.primary.get(&name).and_then(|p| p.test_url.clone());
            self.core.run_proxy_operation(move || {
                let delays = match proxies::test_group_delay(&name, url.as_deref(), timeout) {
                    Ok(delays) => delays,
                    Err(error) => {
                        Confirm::err(error);
                        Default::default()
                    }
                };
                // Failed health checks also update mihomo history; always refresh it.
                let mut response = proxies::fetch_proxies().map_err(|e| e.to_string())?;
                response.record_group_delays(&before, &name, &delays);
                Ok(response)
            })
        } else {
            let proxy = match before.resolve(&name) {
                Ok(proxy) => proxy,
                Err(error) => {
                    self.feedback.push(error.message(&name));
                    return false;
                }
            };
            let target = proxy.target();
            let url = proxy.test_url.map(str::to_owned);
            self.core.run_proxy_operation(move || {
                let delay = match proxies::test_proxy_delay(&target, url.as_deref(), timeout) {
                    Ok(delay) => delay,
                    Err(error) => {
                        Confirm::err(error);
                        None
                    }
                };
                // Failed health checks also update mihomo history; always refresh it.
                let mut response = proxies::fetch_proxies().map_err(|e| e.to_string())?;
                if let Some(delay) = delay {
                    response.record_delay(&target, delay);
                }
                Ok(response)
            })
        };
        self.settle();
        accepted
    }
    pub fn test_all_delay(&mut self, folders: Vec<String>, files: Vec<String>) -> bool {
        let Some(permit) = self.core.api_permit() else {
            return false;
        };
        let total = folders.len() + files.len();
        if total == 0 {
            return false;
        }
        let Some(before) = self.core.queries.proxies.data.clone() else {
            return false;
        };
        let timeout = crate::config::CONFIG.cfg_file.timeout.unwrap_or(5) * 1000;
        let accepted = self.core.run_proxy_operation(move || {
            let mut group_delays = Vec::new();
            let mut node_delays = Vec::new();
            let mut errors = Vec::new();
            for name in folders {
                if !permit.is_valid() || crate::tui::command_cancelled() {
                    return Err("Proxy tests stopped because core API permission expired".into());
                }
                let url = before
                    .primary
                    .get(&name)
                    .and_then(|p| p.test_url.as_deref());
                match proxies::test_group_delay(&name, url, timeout) {
                    Ok(delays) => group_delays.push((name, delays)),
                    Err(error) => errors.push(format!("{name}: {error}")),
                }
            }
            for name in files {
                if !permit.is_valid() || crate::tui::command_cancelled() {
                    return Err("Proxy tests stopped because core API permission expired".into());
                }
                let Ok(proxy) = before.resolve(&name) else {
                    continue;
                };
                let target = proxy.target();
                match proxies::test_proxy_delay(&target, proxy.test_url, timeout) {
                    Ok(Some(delay)) => node_delays.push((target, delay)),
                    Ok(None) => {}
                    Err(error) => errors.push(format!("{name}: {error}")),
                }
            }
            if !permit.is_valid() || crate::tui::command_cancelled() {
                return Err("Proxy tests stopped because core API permission expired".into());
            }
            let mut response = proxies::fetch_proxies().map_err(|e| e.to_string())?;
            for (name, delays) in group_delays {
                response.record_group_delays(&before, &name, &delays);
            }
            for (target, delay) in node_delays {
                response.record_delay(&target, delay);
            }
            if !errors.is_empty() {
                Confirm::err(errors.join("\n"));
            }
            Ok(response)
        });
        self.settle();
        accepted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_commit_updates_hidden_log_state_before_returning() {
        let mut business = Business::default();
        // No App or Logs page participates in this coordination.
        business.pause_logs(true);
        let snapshot = business.view().log_snapshot();
        assert!(snapshot.paused);
        assert_eq!(snapshot.diagnostic.as_deref(), Some("API not checked"));
        assert!(!business.has_work());
    }

    #[test]
    fn shutdown_prevents_leave_and_later_actions_from_starting_work() {
        let mut business = Business::default();
        business.begin_shutdown();
        business.leave(ApiResource::Connections);
        business.enter(ApiResource::Connections);
        business.confirm_api();
        business.check_services();
        assert!(!business.start_service(CoreType::Mihomo, ServiceOperation::Restart));
        assert!(!business.has_work());
        assert!(business.next_deadline().is_none());
        assert_eq!(business.take_feedback().len(), 1);
        assert!(business.take_feedback().is_empty());
    }
}
