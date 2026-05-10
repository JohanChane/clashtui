use super::super::dev::*;
use crate::functions::restful::proxies::{self};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::content::Proxies;
use super::tree::NodeType;

impl Proxies {
    pub fn fzf_find(&mut self, items: Vec<(String, usize)>, task_set: &mut FutureSet<Self>) {
        let names: Vec<String> = items.iter().map(|(name, _)| name.clone()).collect();
        async move {
            let selected = tokio::task::spawn_blocking(move || {
                crate::tui::widget::fzffind::run_fzf(&names, "Find Proxy")
            })
            .await
            .unwrap_or(None);
            // Map fzf positional index back to tree index
            let target = selected.and_then(|pos| items.get(pos).map(|(_, idx)| *idx));
            wrapper(move |content: &mut Self| {
                content.jump_target.set(target);
            })
        }
        .spawn_at(task_set);
    }
}

impl Proxies {
    pub fn select_inline(
        &mut self,
        group: String,
        node: String,
        task_set: &mut FutureSet<Self>,
    ) {
        let t_secs = crate::config::CONFIG.cfg_file.timeout.unwrap_or(5).max(1) + 3;
        self.error = Some(format!("Switching to {node}..."));
        self.testing_since = Some(Instant::now());
        async move {
            let _ = tri!(proxies::select_proxy(&group, &node), or_cancel);
            let response = match tokio::time::timeout(
                Duration::from_secs(t_secs),
                tokio::task::spawn_blocking(|| proxies::fetch_proxies()),
            )
            .await
            {
                Ok(Ok(Ok(r))) => r,
                _ => {
                    return wrapper(move |content: &mut Self| {
                        content.error = None;
                        content.testing_since = None;
                    });
                }
            };
            wrapper(move |content: &mut Self| {
                content.proxies = response.proxies;
                content.tree.rebuild_from_proxies(&content.proxies);
                content.error = None;
                content.testing_since = None;
            })
        }
        .spawn_at(task_set);
    }

    pub fn test_delay(
        &mut self,
        name: String,
        ntype: NodeType,
        task_set: &mut FutureSet<Self>,
    ) {
        let timeout = crate::config::CONFIG.cfg_file.timeout.unwrap_or(5) * 1000;
        let test_url = self.proxies.get(&name)
            .and_then(|p| p.test_url.clone());
        let t_secs = crate::config::CONFIG.cfg_file.timeout.unwrap_or(5).max(1) + 3;

        match ntype {
            NodeType::Folder => {
                self.error = Some(format!("Testing group {name}..."));
                self.testing_since = Some(Instant::now());
                let n = name.clone();
                async move {
                    let delays = match tokio::time::timeout(
                        Duration::from_secs(t_secs),
                        tokio::task::spawn_blocking(move || {
                            proxies::test_group_delay(&n, test_url.as_deref(), timeout)
                        }),
                    )
                    .await
                    {
                        Ok(Ok(Ok(v))) => v,
                        Ok(Ok(Err(e))) => {
                            crate::tui::widget::popmsg::Confirm::err(e);
                            return wrapper(|content: &mut Self| {
                                content.testing_since = None;
                            });
                        }
                        _ => {
                            return wrapper(|content: &mut Self| {
                                content.error = Some("Speed test timed out".to_string());
                                content.testing_since = None;
                            });
                        }
                    };
                    let mut response = match tokio::time::timeout(
                        Duration::from_secs(t_secs),
                        tokio::task::spawn_blocking(|| proxies::fetch_proxies()),
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
                    for (child_name, d) in &delays {
                        if *d > 0 {
                            if let Some(proxy) = response.proxies.get_mut(child_name) {
                                proxy.history.push(proxies::DelayRecord { delay: *d });
                            }
                        }
                    }
                    wrapper(move |content: &mut Self| {
                        content.proxies = response.proxies;
                        content.tree.rebuild_from_proxies(&content.proxies);
                        content.error = None;
                        content.testing_since = None;
                    })
                }
                .spawn_at(task_set);
            }
            _ => {
                self.error = Some(format!("Testing {name}..."));
                self.testing_since = Some(Instant::now());
                let n = name.clone();
                async move {
                    let delay = match tokio::time::timeout(
                        Duration::from_secs(t_secs),
                        tokio::task::spawn_blocking(move || {
                            proxies::test_proxy_delay(&n, test_url.as_deref(), timeout)
                        }),
                    )
                    .await
                    {
                        Ok(Ok(Ok(v))) => v,
                        Ok(Ok(Err(e))) => {
                            let msg = e.to_string();
                            return wrapper(move |content: &mut Self| {
                                content.error = Some(msg);
                                content.testing_since = None;
                            });
                        }
                        _ => {
                            return wrapper(|content: &mut Self| {
                                content.error = Some("Speed test timed out".to_string());
                                content.testing_since = None;
                            });
                        }
                    };
                    let mut response = match tokio::time::timeout(
                        Duration::from_secs(t_secs),
                        tokio::task::spawn_blocking(|| proxies::fetch_proxies()),
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
                    if let (Some(d), Some(proxy)) = (delay, response.proxies.get_mut(&name)) {
                        if d > 0 {
                            proxy.history.push(proxies::DelayRecord { delay: d });
                        }
                    }
                    wrapper(move |content: &mut Self| {
                        content.proxies = response.proxies;
                        content.tree.rebuild_from_proxies(&content.proxies);
                        content.error = None;
                        content.testing_since = None;
                    })
                }
                .spawn_at(task_set);
            }
        }
    }

    pub fn test_all_delay(&mut self, task_set: &mut FutureSet<Self>) {
        let folders: Vec<String> = self.tree.nodes.iter()
            .filter(|n| n.node_type == NodeType::Folder)
            .map(|n| n.name.clone())
            .collect();
        let files: Vec<String> = self.tree.nodes.iter()
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
            let mut all_delays: HashMap<String, u64> = HashMap::new();
            for name in &folders {
                let url = proxies_map.get(name.as_str())
                    .and_then(|p| p.test_url.clone());
                let n = name.clone();
                match tokio::time::timeout(
                    Duration::from_secs(t_secs),
                    tokio::task::spawn_blocking(move || {
                        proxies::test_group_delay(&n, url.as_deref(), timeout)
                    }),
                )
                .await
                {
                    Ok(Ok(Ok(delays))) => all_delays.extend(delays),
                    _ => {}
                }
            }
            for name in &files {
                let url = proxies_map.get(name.as_str())
                    .and_then(|p| p.test_url.clone());
                let n = name.clone();
                match tokio::time::timeout(
                    Duration::from_secs(t_secs),
                    tokio::task::spawn_blocking(move || {
                        proxies::test_proxy_delay(&n, url.as_deref(), timeout)
                    }),
                )
                .await
                {
                    Ok(Ok(Ok(Some(d)))) if d > 0 => {
                        all_delays.insert(name.clone(), d);
                    }
                    _ => {}
                }
            }
            let mut response = match tokio::time::timeout(
                Duration::from_secs(t_secs),
                tokio::task::spawn_blocking(|| proxies::fetch_proxies()),
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
            for (name, d) in &all_delays {
                if *d > 0 {
                    if let Some(proxy) = response.proxies.get_mut(name) {
                        proxy.history.push(proxies::DelayRecord { delay: *d });
                    }
                }
            }
            wrapper(move |content: &mut Self| {
                content.proxies = response.proxies;
                content.tree.rebuild_from_proxies(&content.proxies);
                content.error = None;
                content.testing_since = None;
            })
        }
        .spawn_at(task_set);
    }

    pub fn refresh(&mut self, task_set: &mut FutureSet<Self>) {
        async {
            let response = tri!(proxies::fetch_proxies(), or_set);
            wrapper(move |content: &mut Self| {
                content.proxies = response.proxies;
                content.tree.rebuild_from_proxies(&content.proxies);
                content.error = None;
            })
        }
        .spawn_at(task_set);
    }
}
