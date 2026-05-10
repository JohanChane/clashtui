use super::super::dev::*;
use crate::functions::restful::proxies::{self};
use indexmap::IndexMap;
use std::cell::Cell;
use std::time::Instant;

use super::tree::{NodeType, ProxyTree};

#[derive(Default)]
pub struct Proxies {
    pub tree: ProxyTree,
    pub proxies: IndexMap<String, crate::functions::restful::proxies::Proxy>,
    pub error: Option<String>,
    pub testing_since: Option<Instant>,
    pub jump_target: Cell<Option<usize>>,
}

impl Proxies {
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
                self.tree.sorted = !self.tree.sorted;
                self.tree.sort_by_delay = false;
                self.tree.rebuild_from_proxies(&self.proxies);
            }
            super::Key::SortByDelay => {
                self.tree.sort_by_delay = !self.tree.sort_by_delay;
                self.tree.sorted = false;
                self.tree.rebuild_from_proxies(&self.proxies);
            }
            super::Key::ResetSort => {
                self.tree.sorted = false;
                self.tree.sort_by_delay = false;
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
}

impl TabContent for Proxies {
    fn init(&mut self, task_set: &mut FutureSet<Self>, _state: &mut Self::State) {
        self.error = Some("Loading proxies...".to_owned());
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
