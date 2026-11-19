use super::super::dev::*;
use crate::functions::restful::proxies::{self};
use std::time::{Duration, Instant};

use super::content::Proxies;
use super::tree::{NodeType, ProxyTree};

impl Proxies {
    pub fn fzf_find(&mut self, items: Vec<(String, usize)>, cx: &mut Cx<Self>) {
        self.fzf_find_with_prompt(items, "Find Proxy", cx);
    }

    pub fn fzf_find_with_prompt(
        &mut self,
        items: Vec<(String, usize)>,
        prompt: impl Into<String> + Send + 'static,
        cx: &mut Cx<Self>,
    ) {
        let names: Vec<String> = items.iter().map(|(name, _)| name.clone()).collect();
        let targets: Vec<_> = items
            .iter()
            .map(|(_, index)| self.selection_key(&ListState::default().with_selected(Some(*index))))
            .collect();
        async move {
            let selected = FzfFinder::new(names)
                .with_title(prompt.into())
                .build_and_send()
                .await
                .unwrap_or_default();
            // Map fzf positional index back to tree index
            // Resolve by identity because page refreshes continue while the finder is open.
            let target = selected.and_then(|pos| targets.get(pos).cloned().flatten());
            wrapper(move |content: &mut Self| {
                if target.is_some() {
                    content.filter = None;
                }
                content.jump_target.set(target);
            })
        }
        .spawn_at(cx);
    }
}

impl Proxies {
    pub fn select_inline(&mut self, group: String, node: String, cx: &mut Cx<Self>) {
        if self.selection_busy || self.unavailable() {
            return;
        }
        self.selection_busy = true;
        let is_selector = self
            .proxies
            .primary
            .get(&group)
            .is_some_and(|proxy| proxy.proxy_type.eq_ignore_ascii_case("selector"));
        self.error = Some(format!("Switching to {node}..."));
        self.testing_since = Some(Instant::now());
        let timeout = crate::config::CONFIG.cfg_file.timeout.unwrap_or(5).max(1) + 3;
        async move {
            let selected = tokio::task::spawn_blocking(move || {
                proxies::select_proxy(&group, &node)?;
                Ok::<_, minreq::Error>((group, node))
            })
            .await
            .map_err(anyhow::Error::from)
            .and_then(|result| result.map_err(Into::into));
            let response = if selected.is_ok() {
                match tokio::time::timeout(
                    Duration::from_secs(timeout),
                    tokio::task::spawn_blocking(proxies::fetch_proxies),
                )
                .await
                {
                    Ok(Ok(Ok(response))) => Some(response),
                    _ => None,
                }
            } else {
                None
            };
            wrapper(move |content: &mut Self| {
                content.selection_busy = false;
                content.error = None;
                content.testing_since = None;
                match selected {
                    Ok((group, node)) if is_selector => {
                        content.pending_selections.insert(group, node);
                        content.selected_and_saved = false;
                    }
                    Ok(_) => {}
                    Err(e) => Confirm::err(e),
                }
                if let Some(response) = response {
                    content.proxies = response;
                    content.tree.rebuild_from_proxies(&content.proxies);
                }
            })
        }
        .spawn_at(cx);
    }

    pub(super) fn save_selections(&mut self, cx: &mut Cx<Self>) {
        if self.selection_busy || self.pending_selections.is_empty() || self.unavailable() {
            return;
        }
        let selections = self.pending_selections.clone();
        self.selection_busy = true;
        async move {
            let result = tokio::task::spawn_blocking(move || {
                crate::functions::file::settings::save_proxy_selections(&selections)
            })
            .await
            .map_err(anyhow::Error::from)
            .and_then(|result| result);
            wrapper(move |content: &mut Self| {
                content.selection_busy = false;
                match result {
                    Ok(()) => {
                        content.pending_selections.clear();
                        content.selected_and_saved = true;
                    }
                    Err(e) => Confirm::err(e),
                }
            })
        }
        .spawn_at(cx);
    }

    pub fn test_delay(&mut self, name: String, ntype: NodeType, cx: &mut Cx<Self>) {
        if self.unavailable() {
            return;
        }
        let timeout = crate::config::CONFIG.cfg_file.timeout.unwrap_or(5) * 1000;
        let t_secs = crate::config::CONFIG.cfg_file.timeout.unwrap_or(5).max(1) + 3;

        match ntype {
            NodeType::Folder => {
                let test_url = self
                    .proxies
                    .primary
                    .get(&name)
                    .and_then(|p| p.test_url.clone());
                let before_test = self.proxies.clone();
                self.error = Some(format!("Testing group {name}..."));
                self.testing_since = Some(Instant::now());
                let n = name.clone();
                async move {
                    let (delays, test_error) = match tokio::time::timeout(
                        Duration::from_secs(t_secs),
                        tokio::task::spawn_blocking(move || {
                            proxies::test_group_delay(&n, test_url.as_deref(), timeout)
                        }),
                    )
                    .await
                    {
                        Ok(Ok(Ok(v))) => (v, None),
                        Ok(Ok(Err(e))) => (Default::default(), Some(e.to_string())),
                        _ => (Default::default(), Some("Speed test timed out".to_owned())),
                    };
                    // Failed health checks also update mihomo history; always refresh it.
                    let mut response = match tokio::time::timeout(
                        Duration::from_secs(t_secs),
                        tokio::task::spawn_blocking(proxies::fetch_proxies),
                    )
                    .await
                    {
                        Ok(Ok(Ok(r))) => r,
                        _ => {
                            return wrapper(|content: &mut Self| {
                                content.error =
                                    Some("Failed to refresh proxies after test".to_string());
                                content.testing_since = None;
                            });
                        }
                    };
                    response.record_group_delays(&before_test, &name, &delays);
                    wrapper(move |content: &mut Self| {
                        content.proxies = response;
                        content.tree.rebuild_from_proxies(&content.proxies);
                        content.error = test_error;
                        content.testing_since = None;
                    })
                }
                .spawn_at(cx);
            }
            _ => {
                let resolved = match self.proxies.resolve(&name) {
                    Ok(proxy) => proxy,
                    Err(error) => {
                        self.error = Some(error.message(&name));
                        return;
                    }
                };
                let target = resolved.target();
                let test_url = resolved.test_url.map(str::to_owned);
                self.error = Some(format!("Testing {name}..."));
                self.testing_since = Some(Instant::now());
                let request_target = target.clone();
                async move {
                    let (delay, test_error) = match tokio::time::timeout(
                        Duration::from_secs(t_secs),
                        tokio::task::spawn_blocking(move || {
                            proxies::test_proxy_delay(&request_target, test_url.as_deref(), timeout)
                        }),
                    )
                    .await
                    {
                        Ok(Ok(Ok(v))) => (v, None),
                        Ok(Ok(Err(e))) => (None, Some(e.to_string())),
                        _ => (None, Some("Speed test timed out".to_owned())),
                    };
                    // Failed health checks also update mihomo history; always refresh it.
                    let mut response = match tokio::time::timeout(
                        Duration::from_secs(t_secs),
                        tokio::task::spawn_blocking(proxies::fetch_proxies),
                    )
                    .await
                    {
                        Ok(Ok(Ok(r))) => r,
                        _ => {
                            return wrapper(|content: &mut Self| {
                                content.error =
                                    Some("Failed to refresh proxies after test".to_string());
                                content.testing_since = None;
                            });
                        }
                    };
                    if let Some(delay) = delay {
                        response.record_delay(&target, delay);
                    }
                    wrapper(move |content: &mut Self| {
                        content.proxies = response;
                        content.tree.rebuild_from_proxies(&content.proxies);
                        content.error = test_error;
                        content.testing_since = None;
                    })
                }
                .spawn_at(cx);
            }
        }
    }

    pub fn test_all_delay(&mut self, cx: &mut Cx<Self>) {
        if self.unavailable() {
            return;
        }
        let folders: Vec<String> = self
            .tree
            .nodes
            .iter()
            .filter(|n| n.node_type == NodeType::Folder)
            .map(|n| n.name.clone())
            .collect();
        let files: Vec<String> = self
            .tree
            .nodes
            .iter()
            .filter(|n| n.node_type == NodeType::File && n.depth == 0)
            .map(|n| n.name.clone())
            .collect();
        let total = folders.len() + files.len();
        if total == 0 {
            return;
        }
        let proxies_map = self.proxies.clone();
        let timeout = crate::config::CONFIG.cfg_file.timeout.unwrap_or(5) * 1000;
        self.error = Some(format!("Testing all ({total} groups/nodes)..."));
        self.testing_since = Some(Instant::now());
        async move {
            let t_secs = crate::config::CONFIG.cfg_file.timeout.unwrap_or(5).max(1) + 3;
            let mut group_delays = Vec::new();
            let mut node_delays = Vec::new();
            for name in &folders {
                let url = proxies_map
                    .primary
                    .get(name.as_str())
                    .and_then(|p| p.test_url.clone());
                let n = name.clone();
                if let Ok(Ok(Ok(delays))) = tokio::time::timeout(
                    Duration::from_secs(t_secs),
                    tokio::task::spawn_blocking(move || {
                        proxies::test_group_delay(&n, url.as_deref(), timeout)
                    }),
                )
                .await
                {
                    group_delays.push((name.clone(), delays));
                }
            }
            for name in &files {
                let Ok(proxy) = proxies_map.resolve(name) else {
                    continue;
                };
                let target = proxy.target();
                let request_target = target.clone();
                let url = proxy.test_url.map(str::to_owned);
                if let Ok(Ok(Ok(Some(d)))) = tokio::time::timeout(
                    Duration::from_secs(t_secs),
                    tokio::task::spawn_blocking(move || {
                        proxies::test_proxy_delay(&request_target, url.as_deref(), timeout)
                    }),
                )
                .await
                {
                    node_delays.push((target, d));
                }
            }
            let mut response = match tokio::time::timeout(
                Duration::from_secs(t_secs),
                tokio::task::spawn_blocking(proxies::fetch_proxies),
            )
            .await
            {
                Ok(Ok(Ok(r))) => r,
                _ => {
                    return wrapper(|content: &mut Self| {
                        content.error = Some("Failed to refresh proxies after test".to_string());
                        content.testing_since = None;
                    });
                }
            };
            for (group, delays) in group_delays {
                response.record_group_delays(&proxies_map, &group, &delays);
            }
            for (target, delay) in node_delays {
                response.record_delay(&target, delay);
            }
            wrapper(move |content: &mut Self| {
                content.proxies = response;
                content.tree.rebuild_from_proxies(&content.proxies);
                content.error = None;
                content.testing_since = None;
            })
        }
        .spawn_at(cx);
    }

    fn unavailable(&mut self) -> bool {
        match monitor::status().ready() {
            Ok(()) => false,
            Err(reason) => {
                self.proxies = proxies::ProxyCatalog::default();
                self.tree = ProxyTree::default();
                self.error = Some(reason);
                true
            }
        }
    }

    pub fn reload(&mut self, cx: &mut Cx<Self>) {
        if self.unavailable() {
            return;
        }
        async {
            let response = tri!(
                tokio::task::spawn_blocking(proxies::fetch_proxies)
                    .await
                    .unwrap(),
                or_set
            );
            wrapper(move |content: &mut Self| {
                content.proxies = response;
                content.tree.rebuild_from_proxies(&content.proxies);
                content.error = None;
            })
        }
        .spawn_at(cx);
    }
}
