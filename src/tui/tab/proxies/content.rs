use super::super::dev::*;
use crate::functions::restful::proxies::{self};
use indexmap::IndexMap;
use std::cell::Cell;
use std::time::Instant;

use super::tree::{NodeType, ProxyTree, SortMode};

#[derive(Default)]
pub struct Proxies {
    pub tree: ProxyTree,
    pub proxies: IndexMap<String, crate::functions::restful::proxies::Proxy>,
    pub error: Option<String>,
    pub testing_since: Option<Instant>,
    pub jump_target: Cell<Option<usize>>,
    pub filter: Option<String>,
    pub paused: bool,
}

impl Proxies {
    fn resolve_group_for_sort(&self, cursor: usize) -> Option<String> {
        let node = self.tree.node_at(cursor)?;
        match node.node_type {
            NodeType::Folder => Some(node.name.clone()),
            NodeType::Link | NodeType::File => node.parent.clone(),
        }
    }

    pub fn dispatch_key(
        &mut self,
        key: super::Key,
        task_set: &mut FutureSet<Self>,
        state: &mut ListState,
    ) {
        let current = state.selected().unwrap_or(0);

        match key {
            super::Key::MoveUp => {
                if current > 0 {
                    state.select(Some(current - 1));
                }
            }
            super::Key::MoveDown => {
                if current + 1 < self.tree.len() {
                    state.select(Some(current + 1));
                }
            }
            super::Key::GoTop => {
                if self.tree.len() > 0 {
                    state.select(Some(0));
                }
            }
            super::Key::GoBottom => {
                if self.tree.len() > 0 {
                    state.select(Some(self.tree.len().saturating_sub(1)));
                }
            }
            super::Key::Parent => {
                let info = self.tree.node_at(current)
                    .map(|n| (n.name.clone(), n.node_type.clone(), n.parent.clone()));
                if let Some((name, ntype, parent)) = info {
                    match ntype {
                        NodeType::Folder => {
                            self.tree.collapse_at(&name, &self.proxies);
                            if let Some(idx) = self.tree.find_folder_index(&name) {
                                state.select(Some(idx));
                            }
                        }
                        _ => {
                            if let Some(ref parent) = parent {
                                self.tree.collapse_at(parent, &self.proxies);
                                if let Some(idx) = self.tree.find_folder_index(parent) {
                                    state.select(Some(idx));
                                }
                            }
                        }
                    }
                }
            }
            super::Key::Expand => {
                let info = self.tree.node_at(current)
                    .map(|n| (n.name.clone(), n.node_type.clone(), n.parent.clone()));
                if let Some((name, ntype, _parent)) = info {
                    match ntype {
                        NodeType::Folder => self.tree.expand_at(&name, &self.proxies),
                        NodeType::Link => {
                            if let Some(idx) = self.tree.find_folder_index(&name) {
                                state.select(Some(idx));
                            }
                        }
                        NodeType::File => {}
                    }
                }
            }
            super::Key::Select => {
                let info = self.tree.node_at(current)
                    .map(|n| (n.name.clone(), n.node_type.clone(), n.parent.clone()));
                if let Some((name, ntype, parent)) = info {
                    match ntype {
                        NodeType::Folder => {
                            self.tree.toggle_expand_at(&name, &self.proxies);
                        }
                        NodeType::Link | NodeType::File => {
                            if let Some(ref parent) = parent {
                                self.select_inline(parent.clone(), name, task_set);
                            }
                        }
                    }
                }
            }
            super::Key::CollapseAll => self.tree.collapse_all(&self.proxies),
            super::Key::ExpandAll => self.tree.expand_all(&self.proxies),
            super::Key::Refresh => self.refresh(task_set),
            super::Key::SortByName => {
                if let Some(group_name) = self.resolve_group_for_sort(current) {
                    if let Some(idx) = self.tree.find_folder_index(&group_name) {
                        let new_mode = if self.tree.nodes[idx].sort_mode == SortMode::ByName {
                            SortMode::None
                        } else {
                            SortMode::ByName
                        };
                        self.tree.nodes[idx].sort_mode = new_mode;
                        self.tree.rebuild_from_proxies(&self.proxies);
                    }
                }
            }
            super::Key::SortByDelay => {
                if let Some(group_name) = self.resolve_group_for_sort(current) {
                    if let Some(idx) = self.tree.find_folder_index(&group_name) {
                        let new_mode = if self.tree.nodes[idx].sort_mode == SortMode::ByDelay {
                            SortMode::None
                        } else {
                            SortMode::ByDelay
                        };
                        self.tree.nodes[idx].sort_mode = new_mode;
                        self.tree.rebuild_from_proxies(&self.proxies);
                    }
                }
            }
            super::Key::ResetSort => {
                if let Some(group_name) = self.resolve_group_for_sort(current) {
                    if let Some(idx) = self.tree.find_folder_index(&group_name) {
                        self.tree.nodes[idx].sort_mode = SortMode::None;
                        self.tree.rebuild_from_proxies(&self.proxies);
                    }
                }
            }
            super::Key::GlobalSortByName => {
                let all_by_name = self.tree.nodes.iter()
                    .filter(|n| n.node_type == NodeType::Folder)
                    .all(|n| n.sort_mode == SortMode::ByName);
                let new_mode = if all_by_name { SortMode::None } else { SortMode::ByName };
                for node in &mut self.tree.nodes {
                    if node.node_type == NodeType::Folder {
                        node.sort_mode = new_mode;
                    }
                }
                self.tree.rebuild_from_proxies(&self.proxies);
            }
            super::Key::GlobalSortByDelay => {
                let all_by_delay = self.tree.nodes.iter()
                    .filter(|n| n.node_type == NodeType::Folder)
                    .all(|n| n.sort_mode == SortMode::ByDelay);
                let new_mode = if all_by_delay { SortMode::None } else { SortMode::ByDelay };
                for node in &mut self.tree.nodes {
                    if node.node_type == NodeType::Folder {
                        node.sort_mode = new_mode;
                    }
                }
                self.tree.rebuild_from_proxies(&self.proxies);
            }
            super::Key::GlobalResetSort => {
                for node in &mut self.tree.nodes {
                    if node.node_type == NodeType::Folder {
                        node.sort_mode = SortMode::None;
                    }
                }
                self.tree.rebuild_from_proxies(&self.proxies);
            }
            super::Key::TestDelay => {
                let info = self.tree.node_at(current)
                    .map(|n| (n.name.clone(), n.node_type.clone()));
                if let Some((name, ntype)) = info {
                    self.test_delay(name, ntype, task_set);
                }
            }
            super::Key::TestAllDelay => self.test_all_delay(task_set),
            super::Key::Search => {
                async move {
                    let filter = tri!(
                        Input::new()
                            .with_title("Filter".to_owned())
                            .build_and_send()
                            .await,
                        or_cancel
                    );
                    wrapper(move |content: &mut Self| {
                        content.filter = (!filter.is_empty()).then_some(filter);
                    })
                }
                .spawn_at(task_set);
            }
            super::Key::FzfFind => {
                let items: Vec<(String, usize)> = self.tree.nodes.iter()
                    .enumerate()
                    .map(|(i, n)| (n.name.clone(), i))
                    .collect();
                self.fzf_find(items, task_set);
            }
        }
    }
}

impl BasicTabContent for Proxies {
    type Key = super::Key;
    type State = ListState;

    const TITLE: &str = "Proxies";

    fn all_shortcuts() -> &'static [(KeyCombo, Self::Key, &'static str)] {
        super::agent::all_shortcuts()
    }

    fn after_sync(&self, task_set: &mut FutureSet<Self>) {
        if self.paused {
            return;
        }
        if crate::config::is_core_mismatch() {
            return;
        }
        async {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            let response = tri!(proxies::fetch_proxies(), or_set);
            wrapper(|content: &mut Self| {
                content.proxies = response.proxies;
                content.tree.rebuild_from_proxies(&content.proxies);
                content.error = None;
            })
        }
        .spawn_at(task_set);
    }

    fn on_enter(&mut self, task_set: &mut FutureSet<Self>, _state: &mut Self::State) {
        self.paused = false;
        if crate::config::is_core_mismatch() {
            self.proxies = IndexMap::new();
            self.tree = ProxyTree::default();
            self.error = Some("API data mismatch with configured core".to_owned());
            return;
        }
        async {
            let response = tri!(proxies::fetch_proxies());
            wrapper(|content: &mut Self| {
                content.proxies = response.proxies.clone();
                content.tree = ProxyTree::build(response);
                content.error = None;
            })
        }
        .spawn_at(task_set);
    }

    fn on_leave(&mut self, _task_set: &mut FutureSet<Self>, _state: &mut Self::State) {
        self.paused = true;
    }
}

impl TabContent for Proxies {
    fn init(&mut self, _task_set: &mut FutureSet<Self>, _state: &mut Self::State) {
        self.paused = true;
        self.error = Some("Loading proxies...".to_owned());
    }

    fn handle_key_event(
        &mut self,
        key: super::Key,
        task_set: &mut FutureSet<Self>,
        state: &mut Self::State,
    ) {
        self.dispatch_key(key, task_set, state);
    }

    fn render(&self, f: &mut Frame, area: Rect, state: &mut Self::State) {
        if let Some(idx) = self.jump_target.take() {
            if idx < self.tree.len() {
                state.select(Some(idx));
            }
        }
        super::render::render(self, f, area, state);
    }
}
