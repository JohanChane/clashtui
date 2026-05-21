use super::super::dev::*;
use crate::functions::restful::proxies::{self};
use indexmap::IndexMap;
use std::cell::Cell;
use std::time::Instant;

use super::tree::{NodeItem, NodeType, ProxyTree, SortMode};

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

type SelectionKey = (String, Option<String>, NodeType);

impl Proxies {
    fn resolve_group_for_sort(&self, cursor: usize) -> Option<String> {
        let node = self.tree.node_at(cursor)?;
        match node.node_type {
            NodeType::Folder => Some(node.name.clone()),
            NodeType::Link | NodeType::File => node.parent.clone(),
        }
    }

    fn fzf_display(node: &NodeItem) -> String {
        let mut s = node.name.clone();
        if !node.proxy_type.is_empty() {
            s.push_str(&format!("  [{}]", node.proxy_type));
        }
        if node.node_type != NodeType::Folder {
            if node.tcp {
                s.push_str(" TCP");
            }
            if node.udp {
                s.push_str(" UDP");
            }
        }
        s
    }

    pub(crate) fn selection_key(&self, state: &ListState) -> Option<SelectionKey> {
        state
            .selected()
            .and_then(|i| self.tree.node_at(i))
            .map(|n| (n.name.clone(), n.parent.clone(), n.node_type.clone()))
    }

    pub(crate) fn restore_selection(&self, key: Option<SelectionKey>, state: &mut ListState) {
        state.select(None);
        if let Some((name, parent, ntype)) = key {
            if let Some(idx) = self
                .tree
                .nodes
                .iter()
                .position(|n| n.name == name && n.parent == parent && n.node_type == ntype)
            {
                state.select(Some(idx));
            }
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
                let info = self
                    .tree
                    .node_at(current)
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
                let info = self
                    .tree
                    .node_at(current)
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
                let info = self
                    .tree
                    .node_at(current)
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
            super::Key::CollapseAll => {
                let key = self.selection_key(state);
                self.tree.collapse_all(&self.proxies);
                self.restore_selection(key, state);
            }
            super::Key::ExpandAll => {
                let key = self.selection_key(state);
                self.tree.expand_all(&self.proxies);
                self.restore_selection(key, state);
            }
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
                        let key = self.selection_key(state);
                        self.tree.rebuild_from_proxies(&self.proxies);
                        self.restore_selection(key, state);
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
                        let key = self.selection_key(state);
                        self.tree.rebuild_from_proxies(&self.proxies);
                        self.restore_selection(key, state);
                    }
                }
            }
            super::Key::ResetSort => {
                if let Some(group_name) = self.resolve_group_for_sort(current) {
                    if let Some(idx) = self.tree.find_folder_index(&group_name) {
                        self.tree.nodes[idx].sort_mode = SortMode::None;
                        let key = self.selection_key(state);
                        self.tree.rebuild_from_proxies(&self.proxies);
                        self.restore_selection(key, state);
                    }
                }
            }
            super::Key::GlobalSortByName => {
                let all_by_name = self
                    .tree
                    .nodes
                    .iter()
                    .filter(|n| n.node_type == NodeType::Folder)
                    .all(|n| n.sort_mode == SortMode::ByName);
                let new_mode = if all_by_name {
                    SortMode::None
                } else {
                    SortMode::ByName
                };
                for node in &mut self.tree.nodes {
                    if node.node_type == NodeType::Folder {
                        node.sort_mode = new_mode;
                    }
                }
                let key = self.selection_key(state);
                self.tree.rebuild_from_proxies(&self.proxies);
                self.restore_selection(key, state);
            }
            super::Key::GlobalSortByDelay => {
                let all_by_delay = self
                    .tree
                    .nodes
                    .iter()
                    .filter(|n| n.node_type == NodeType::Folder)
                    .all(|n| n.sort_mode == SortMode::ByDelay);
                let new_mode = if all_by_delay {
                    SortMode::None
                } else {
                    SortMode::ByDelay
                };
                for node in &mut self.tree.nodes {
                    if node.node_type == NodeType::Folder {
                        node.sort_mode = new_mode;
                    }
                }
                let key = self.selection_key(state);
                self.tree.rebuild_from_proxies(&self.proxies);
                self.restore_selection(key, state);
            }
            super::Key::GlobalResetSort => {
                for node in &mut self.tree.nodes {
                    if node.node_type == NodeType::Folder {
                        node.sort_mode = SortMode::None;
                    }
                }
                let key = self.selection_key(state);
                self.tree.rebuild_from_proxies(&self.proxies);
                self.restore_selection(key, state);
            }
            super::Key::TestDelay => {
                let info = self
                    .tree
                    .node_at(current)
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
                let items: Vec<(String, usize)> = self
                    .tree
                    .nodes
                    .iter()
                    .enumerate()
                    .map(|(i, n)| (Self::fzf_display(n), i))
                    .collect();
                self.fzf_find(items, task_set);
            }
            super::Key::GroupSelect => {
                let node = self.tree.node_at(current);
                let parent = node.map(|n| n.parent.clone()).flatten();
                let group_name = parent.as_deref().unwrap_or("top");
                let prompt = format!("Select in {group_name}");
                let items: Vec<(String, usize)> = self
                    .tree
                    .nodes
                    .iter()
                    .enumerate()
                    .filter(|(_, n)| n.parent == parent)
                    .map(|(i, n)| (Self::fzf_display(n), i))
                    .collect();
                if !items.is_empty() {
                    self.fzf_find_with_prompt(items, &prompt, task_set);
                }
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
            let response = tri!(
                tokio::task::spawn_blocking(proxies::fetch_proxies)
                    .await
                    .unwrap(),
                or_set
            );
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
            let response = tri!(
                tokio::task::spawn_blocking(proxies::fetch_proxies)
                    .await
                    .unwrap()
            );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::functions::restful::proxies::ProxiesResponse;
    use ratatui::widgets::ListState;

    fn load_fixture() -> (Proxies, ListState) {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/tui/tab/proxies/tests/fixtures/proxies.json"
        );
        let data = std::fs::read_to_string(path).unwrap();
        let response: ProxiesResponse = serde_json::from_str(&data).unwrap();
        let content = Proxies {
            tree: ProxyTree::build(ProxiesResponse {
                proxies: response.proxies.clone(),
            }),
            proxies: response.proxies,
            ..Default::default()
        };
        let mut state = ListState::default();
        state.select(Some(0));
        (content, state)
    }

    #[test]
    fn selection_key_returns_folder_identity() {
        let (content, mut state) = load_fixture();
        let folder_idx = content
            .tree
            .nodes
            .iter()
            .position(|n| n.node_type == NodeType::Folder && n.name == "Sl-pvd0")
            .unwrap();
        state.select(Some(folder_idx));
        let key = content.selection_key(&state).unwrap();
        assert_eq!(key.0, "Sl-pvd0");
        assert_eq!(key.2, NodeType::Folder);
    }

    #[test]
    fn selection_key_none_when_no_selection() {
        let (content, state) = load_fixture();
        let mut s = state.clone();
        s.select(None);
        assert!(content.selection_key(&s).is_none());
    }

    #[test]
    fn restore_selection_finds_node() {
        let (content, mut state) = load_fixture();
        let folder_idx = content
            .tree
            .nodes
            .iter()
            .position(|n| n.node_type == NodeType::Folder && n.name == "Sl-pvd0")
            .unwrap();
        let node = content.tree.node_at(folder_idx).unwrap();
        let key = (
            node.name.clone(),
            node.parent.clone(),
            node.node_type.clone(),
        );
        state.select(None);
        content.restore_selection(Some(key), &mut state);
        assert_eq!(state.selected(), Some(folder_idx));
    }

    #[test]
    fn restore_selection_none_clears_selection() {
        let (content, mut state) = load_fixture();
        content.restore_selection(None, &mut state);
        assert!(state.selected().is_none());
    }

    #[test]
    fn resolve_group_for_sort_returns_folder_name() {
        let (content, _) = load_fixture();
        let folder_idx = content
            .tree
            .nodes
            .iter()
            .position(|n| n.node_type == NodeType::Folder && n.name == "Entry")
            .unwrap();
        let group = content.resolve_group_for_sort(folder_idx).unwrap();
        assert_eq!(group, "Entry");
    }

    #[test]
    fn resolve_group_for_sort_returns_parent_for_child() {
        use crate::tui::widget::tab::{FutureSet, wrapper};
        let (mut content, mut state) = load_fixture();

        // Expand the first folder to reveal its children
        let folder_idx = content
            .tree
            .nodes
            .iter()
            .position(|n| n.node_type == NodeType::Folder && n.name == "Entry")
            .unwrap();
        state.select(Some(folder_idx));
        let key = content.selection_key(&state).unwrap();
        content.tree.expand_at("Entry", &content.proxies);
        content.tree.rebuild_from_proxies(&content.proxies);
        content.restore_selection(Some(key), &mut state);

        // Now find a child under Entry
        let child_idx = content
            .tree
            .nodes
            .iter()
            .position(|n| n.name == "Sl-pvd0" && n.parent.as_deref() == Some("Entry"))
            .expect("Sl-pvd0 should exist as child of Entry after expand");
        let group = content.resolve_group_for_sort(child_idx).unwrap();
        assert_eq!(group, "Entry");
    }

    #[test]
    fn fzf_display_includes_proxy_type() {
        let node = NodeItem {
            name: "test-proxy".into(),
            node_type: NodeType::Link,
            parent: None,
            proxy_type: "vmess".into(),
            depth: 0,
            delay: None,
            expanded: false,
            is_now: false,
            sort_mode: SortMode::None,
            tcp: false,
            udp: false,
        };
        let display = Proxies::fzf_display(&node);
        assert!(display.contains("[vmess]"));
        assert!(display.contains("test-proxy"));
    }

    #[test]
    fn fzf_display_includes_tcp_udp() {
        let node = NodeItem {
            name: "test-proxy".into(),
            node_type: NodeType::Link,
            parent: None,
            proxy_type: "".into(),
            depth: 0,
            delay: None,
            expanded: false,
            is_now: false,
            sort_mode: SortMode::None,
            tcp: true,
            udp: true,
        };
        let display = Proxies::fzf_display(&node);
        assert!(display.contains("TCP"));
        assert!(display.contains("UDP"));
    }

    #[test]
    fn fzf_display_folder_omits_tcp_udp() {
        let node = NodeItem {
            name: "TestGroup".into(),
            node_type: NodeType::Folder,
            parent: None,
            proxy_type: "".into(),
            depth: 0,
            delay: None,
            expanded: false,
            is_now: false,
            sort_mode: SortMode::None,
            tcp: true,
            udp: false,
        };
        let display = Proxies::fzf_display(&node);
        assert!(!display.contains("TCP"));
        assert!(!display.contains("UDP"));
    }
}
