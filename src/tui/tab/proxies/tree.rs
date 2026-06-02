use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum SortMode {
    #[default]
    None,
    ByName,
    ByDelay,
}

#[derive(Clone, PartialEq, Debug)]
pub enum NodeType {
    Folder,
    Link,
    File,
}

#[derive(Clone)]
pub struct NodeItem {
    pub name: String,
    pub depth: usize,
    pub node_type: NodeType,
    pub proxy_type: String,
    pub delay: Option<u64>,
    pub parent: Option<String>,
    pub expanded: bool,
    pub is_now: bool,
    pub sort_mode: SortMode,
    pub tcp: bool,
    pub udp: bool,
}

pub struct ProxyTree {
    pub nodes: Vec<NodeItem>,
    pub name_index: HashMap<String, usize>,
}

impl Default for ProxyTree {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            name_index: HashMap::new(),
        }
    }
}

impl ProxyTree {
    pub fn build(response: crate::functions::restful::proxies::ProxiesResponse) -> Self {
        let proxies = response.proxies;
        let mut tree = ProxyTree::default();
        tree.rebuild_from_proxies(&proxies);
        tree
    }

    pub fn rebuild_from_proxies(
        &mut self,
        proxies: &IndexMap<String, crate::functions::restful::proxies::Proxy>,
    ) {
        let expanded_map: HashMap<String, bool> = self
            .nodes
            .iter()
            .filter(|n| n.expanded && n.node_type == NodeType::Folder)
            .map(|n| (n.name.clone(), true))
            .collect();

        let sort_map: HashMap<String, SortMode> = self
            .nodes
            .iter()
            .filter(|n| n.node_type == NodeType::Folder && n.sort_mode != SortMode::None)
            .map(|n| (n.name.clone(), n.sort_mode))
            .collect();

        let mut nodes = Vec::new();

        let mut top: Vec<&str> = proxies
            .iter()
            .filter(|(_, p)| !p.hidden && p.all.as_ref().map(|a| !a.is_empty()).unwrap_or(false))
            .map(|(name, _)| name.as_str())
            .collect();

        let global_sort = sort_map.get("GLOBAL").copied().unwrap_or(SortMode::None);
        if global_sort == SortMode::ByName {
            top.sort();
        } else if let Some(global) = proxies.get("GLOBAL") {
            if let Some(ref group_all) = global.all {
                let sort_index: Vec<&str> = group_all.iter().map(|s| s.as_str()).collect();
                top.sort_by_key(|name| {
                    if *name == "GLOBAL" {
                        usize::MAX
                    } else {
                        sort_index
                            .iter()
                            .position(|&s| s == *name)
                            .unwrap_or(usize::MAX - 1)
                    }
                });
            }
        }

        for name in &top {
            let sort_mode = sort_map.get(*name).copied().unwrap_or(SortMode::None);
            Self::push_entry(
                &mut nodes,
                name,
                None,
                None,
                0,
                proxies,
                &expanded_map,
                sort_mode,
            );
        }

        self.nodes = nodes;
        self.rebuild_index();
    }

    pub fn push_entry(
        nodes: &mut Vec<NodeItem>,
        name: &str,
        parent: Option<String>,
        parent_now: Option<&str>,
        depth: usize,
        proxies: &IndexMap<String, crate::functions::restful::proxies::Proxy>,
        expanded_map: &HashMap<String, bool>,
        sort_mode: SortMode,
    ) {
        let proxy = match proxies.get(name) {
            Some(p) => p,
            None => return,
        };
        if proxy.hidden {
            return;
        }
        let has_kids = proxy.all.as_ref().map(|a| !a.is_empty()).unwrap_or(false);
        let node_type = if has_kids {
            NodeType::Folder
        } else {
            NodeType::File
        };
        let expanded = expanded_map.get(name).copied().unwrap_or(false);

        nodes.push(NodeItem {
            name: name.to_owned(),
            depth,
            node_type,
            proxy_type: proxy.proxy_type.clone(),
            delay: None,
            parent,
            expanded,
            is_now: parent_now == Some(name),
            sort_mode,
            tcp: proxy.tcp,
            udp: proxy.udp,
        });

        if has_kids && expanded {
            if let Some(ref kids) = proxy.all {
                let my_now = proxy.now.as_deref();
                let ordered_kids: Vec<&String> = match sort_mode {
                    SortMode::ByDelay => {
                        let mut v: Vec<&String> = kids.iter().collect();
                        v.sort_by_key(|kid| {
                            resolve_delay(kid.as_str(), proxies)
                                .and_then(|d| if d == 0 { None } else { Some(d) })
                                .unwrap_or(u64::MAX)
                        });
                        v
                    }
                    SortMode::ByName => {
                        let mut v: Vec<&String> = kids.iter().collect();
                        v.sort();
                        v
                    }
                    SortMode::None => kids.iter().collect(),
                };
                for kid in &ordered_kids {
                    let is_group = proxies
                        .get(kid.as_str())
                        .map(|p| p.all.as_ref().map(|a| !a.is_empty()).unwrap_or(false))
                        .unwrap_or(false);
                    let ntype = if is_group {
                        NodeType::Link
                    } else {
                        NodeType::File
                    };
                    let kid_proxy = proxies.get(kid.as_str());
                    nodes.push(NodeItem {
                        name: (*kid).clone(),
                        depth: depth + 1,
                        node_type: ntype,
                        proxy_type: kid_proxy.map(|p| p.proxy_type.clone()).unwrap_or_default(),
                        delay: resolve_delay(kid.as_str(), proxies),
                        parent: Some(name.to_owned()),
                        expanded: false,
                        is_now: my_now == Some(kid.as_str()),
                        sort_mode: SortMode::None,
                        tcp: kid_proxy.map(|p| p.tcp).unwrap_or(false),
                        udp: kid_proxy.map(|p| p.udp).unwrap_or(false),
                    });
                }
            }
        }
    }

    pub fn toggle_expand_at(
        &mut self,
        name: &str,
        proxies: &IndexMap<String, crate::functions::restful::proxies::Proxy>,
    ) {
        if let Some(idx) = self.find_folder_index(name) {
            self.nodes[idx].expanded = !self.nodes[idx].expanded;
            self.rebuild_from_proxies(proxies);
        }
    }

    pub fn expand_at(
        &mut self,
        name: &str,
        proxies: &IndexMap<String, crate::functions::restful::proxies::Proxy>,
    ) {
        if let Some(idx) = self.find_folder_index(name) {
            self.nodes[idx].expanded = true;
            self.rebuild_from_proxies(proxies);
        }
    }

    pub fn collapse_at(
        &mut self,
        name: &str,
        proxies: &IndexMap<String, crate::functions::restful::proxies::Proxy>,
    ) {
        if let Some(idx) = self.find_folder_index(name) {
            self.nodes[idx].expanded = false;
            self.rebuild_from_proxies(proxies);
        }
    }

    pub fn collapse_all(
        &mut self,
        proxies: &IndexMap<String, crate::functions::restful::proxies::Proxy>,
    ) {
        for n in &mut self.nodes {
            n.expanded = false;
        }
        self.rebuild_from_proxies(proxies);
    }

    pub fn expand_all(
        &mut self,
        proxies: &IndexMap<String, crate::functions::restful::proxies::Proxy>,
    ) {
        for n in &mut self.nodes {
            if n.node_type == NodeType::Folder {
                n.expanded = true;
            }
        }
        self.rebuild_from_proxies(proxies);
    }

    pub fn find_folder_index(&self, name: &str) -> Option<usize> {
        self.nodes
            .iter()
            .position(|n| n.node_type == NodeType::Folder && n.name == name)
    }

    pub fn rebuild_index(&mut self) {
        self.name_index.clear();
        for (i, node) in self.nodes.iter().enumerate() {
            self.name_index.insert(node.name.clone(), i);
        }
    }

    pub fn node_at(&self, idx: usize) -> Option<&NodeItem> {
        self.nodes.get(idx)
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}

fn pick_delay(r: Option<&crate::functions::restful::proxies::DelayRecord>) -> Option<u64> {
    r.and_then(|r| if r.delay > 0 { Some(r.delay) } else { None })
}

pub fn resolve_delay(
    name: &str,
    proxies: &IndexMap<String, crate::functions::restful::proxies::Proxy>,
) -> Option<u64> {
    let proxy = proxies.get(name)?;
    if let Some(d) = pick_delay(proxy.history.last()) {
        return Some(d);
    }
    let has_kids = proxy.all.as_ref().map(|a| !a.is_empty()).unwrap_or(false);
    if has_kids {
        if let Some(d) = resolve_now_delay(name, proxies) {
            return Some(d);
        }
    }
    if !proxy.history.is_empty() {
        return Some(0);
    }
    None
}

fn resolve_now_delay(
    start: &str,
    proxies: &IndexMap<String, crate::functions::restful::proxies::Proxy>,
) -> Option<u64> {
    let mut current = start.to_string();
    let mut visited = HashSet::new();
    loop {
        if !visited.insert(current.clone()) {
            return None;
        }
        let proxy = proxies.get(current.as_str())?;
        let has_kids = proxy.all.as_ref().map(|a| !a.is_empty()).unwrap_or(false);
        if !has_kids {
            return pick_delay(proxy.history.last());
        }
        match proxy.now.as_deref() {
            Some(now) if now != current => current = now.to_string(),
            _ => return pick_delay(proxy.history.last()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::functions::restful::proxies::{self, ProxiesResponse, Proxy};
    use indexmap::IndexMap;

    fn load_fixture() -> IndexMap<String, Proxy> {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/apidata/mihomo/proxies.json"
        );
        let data = std::fs::read_to_string(path).expect("Failed to read fixture");
        let response: ProxiesResponse =
            serde_json::from_str(&data).expect("Failed to parse fixture");
        response.proxies
    }

    #[test]
    fn tree_builds_all_top_level_groups() {
        let proxies = load_fixture();
        let tree = ProxyTree::build(ProxiesResponse {
            proxies: proxies.clone(),
        });
        let top_count = proxies
            .iter()
            .filter(|(_, p)| !p.hidden && p.all.as_ref().map(|a| !a.is_empty()).unwrap_or(false))
            .count();
        let folder_count = tree
            .nodes
            .iter()
            .filter(|n| n.node_type == NodeType::Folder)
            .count();
        assert_eq!(
            folder_count, top_count,
            "Every top-level group becomes a Folder"
        );
    }

    #[test]
    fn resolve_delay_follows_now_chain() {
        let proxies = load_fixture();
        let delay = resolve_delay("Sl-hajimi", &proxies);
        assert!(delay.is_some(), "Sl-hajimi Selector should resolve a delay");
        assert!(delay.unwrap() > 0);
    }

    #[test]
    fn selector_group_resolves_via_now_chain() {
        let proxies = load_fixture();
        let delay = resolve_delay("Sl-manbo", &proxies);
        assert!(delay.is_some(), "Selector should resolve via now chain");
    }

    #[test]
    fn url_test_group_resolves_delay() {
        let proxies = load_fixture();
        let delay = resolve_delay("At-hajimi", &proxies);
        assert!(delay.is_some(), "URLTest should have delay");
        assert!(delay.unwrap() > 0);
    }

    #[test]
    fn leaf_proxy_uses_own_history() {
        let proxies = load_fixture();
        let delay = resolve_delay("日本-优化", &proxies);
        assert!(delay.is_some(), "Leaf VMess should use own history");
        assert!(delay.unwrap() > 0);
    }

    #[test]
    fn zero_delay_history_shows_fail() {
        let proxies = load_fixture();
        assert_eq!(
            resolve_delay("台湾-优化3", &proxies),
            Some(0),
            "All-zero history = FAIL"
        );
    }

    #[test]
    fn missing_proxy_returns_none() {
        let proxies = load_fixture();
        assert_eq!(resolve_delay("nonexistent", &proxies), None);
    }

    #[test]
    fn cycle_detection_returns_none() {
        let mut proxies = IndexMap::new();
        proxies.insert(
            "A".to_string(),
            Proxy {
                name: "A".to_string(),
                now: Some("B".to_string()),
                all: Some(vec!["B".to_string()]),
                ..Default::default()
            },
        );
        proxies.insert(
            "B".to_string(),
            Proxy {
                name: "B".to_string(),
                now: Some("A".to_string()),
                all: Some(vec!["A".to_string()]),
                ..Default::default()
            },
        );
        assert_eq!(resolve_now_delay("A", &proxies), None);
    }

    #[test]
    fn node_types_are_correct() {
        let proxies = load_fixture();
        let tree = ProxyTree::build(ProxiesResponse {
            proxies: proxies.clone(),
        });
        for node in &tree.nodes {
            match node.node_type {
                NodeType::Folder => assert!(
                    node.parent.is_none(),
                    "Folder {} should be top-level",
                    node.name
                ),
                NodeType::Link | NodeType::File => {
                    assert!(
                        node.parent.is_some(),
                        "{:?} {} should have parent",
                        node.node_type,
                        node.name
                    );
                    assert!(
                        node.depth > 0,
                        "{:?} {} should be nested",
                        node.node_type,
                        node.name
                    );
                }
            }
        }
    }

    #[test]
    fn expanded_folder_has_nested_children() {
        let proxies = load_fixture();
        let child_count = proxies
            .get("Entry")
            .unwrap()
            .all
            .as_ref()
            .map(|a| a.len())
            .unwrap();
        assert!(child_count > 0);

        let mut nodes = Vec::new();
        let mut expanded = HashMap::new();
        expanded.insert("Entry".to_string(), true);
        ProxyTree::push_entry(
            &mut nodes,
            "Entry",
            None,
            None,
            0,
            &proxies,
            &expanded,
            SortMode::None,
        );

        let children: Vec<_> = nodes.iter().skip(1).filter(|n| n.depth == 1).collect();
        assert_eq!(children.len(), child_count);
    }

    #[test]
    fn empty_history_returns_none() {
        let mut proxies = IndexMap::new();
        proxies.insert(
            "empty".to_string(),
            Proxy {
                name: "empty".to_string(),
                proxy_type: "Vmess".to_string(),
                history: vec![],
                ..Default::default()
            },
        );
        assert_eq!(resolve_delay("empty", &proxies), None);
    }

    #[test]
    fn zero_delay_produces_fail() {
        let mut proxies = IndexMap::new();
        proxies.insert(
            "dead".to_string(),
            Proxy {
                name: "dead".to_string(),
                proxy_type: "Vmess".to_string(),
                history: vec![proxies::DelayRecord { delay: 0 }],
                ..Default::default()
            },
        );
        assert_eq!(resolve_delay("dead", &proxies), Some(0));
    }

    #[test]
    fn now_chain_multiple_levels() {
        let mut proxies = IndexMap::new();
        proxies.insert(
            "G1".to_string(),
            Proxy {
                name: "G1".to_string(),
                proxy_type: "Selector".to_string(),
                now: Some("G2".to_string()),
                all: Some(vec!["G2".to_string()]),
                ..Default::default()
            },
        );
        proxies.insert(
            "G2".to_string(),
            Proxy {
                name: "G2".to_string(),
                proxy_type: "Selector".to_string(),
                now: Some("LEAF".to_string()),
                all: Some(vec!["LEAF".to_string()]),
                ..Default::default()
            },
        );
        proxies.insert(
            "LEAF".to_string(),
            Proxy {
                name: "LEAF".to_string(),
                proxy_type: "Vmess".to_string(),
                history: vec![proxies::DelayRecord { delay: 42 }],
                ..Default::default()
            },
        );
        assert_eq!(resolve_now_delay("G1", &proxies), Some(42));
        assert_eq!(resolve_delay("G1", &proxies), Some(42));
    }

    #[test]
    fn now_chain_self_ref_stops() {
        let mut proxies = IndexMap::new();
        proxies.insert(
            "self-ref".to_string(),
            Proxy {
                name: "self-ref".to_string(),
                proxy_type: "Selector".to_string(),
                now: Some("self-ref".to_string()),
                all: Some(vec!["self-ref".to_string()]),
                history: vec![proxies::DelayRecord { delay: 99 }],
                ..Default::default()
            },
        );
        assert_eq!(resolve_now_delay("self-ref", &proxies), Some(99));
    }

    #[test]
    fn node_item_populates_udp_and_tcp_from_proxy() {
        let mut proxies = IndexMap::new();
        proxies.insert(
            "leaf".to_string(),
            Proxy {
                name: "leaf".to_string(),
                proxy_type: "Vmess".to_string(),
                udp: true,
                tcp: true,
                ..Default::default()
            },
        );

        let mut nodes = Vec::new();
        ProxyTree::push_entry(
            &mut nodes,
            "leaf",
            None,
            None,
            0,
            &proxies,
            &HashMap::new(),
            SortMode::None,
        );

        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].udp, true);
        assert_eq!(nodes[0].tcp, true);
    }

    #[test]
    fn node_item_populates_protocol_false_from_proxy() {
        let mut proxies = IndexMap::new();
        proxies.insert(
            "tcp_only".to_string(),
            Proxy {
                name: "tcp_only".to_string(),
                proxy_type: "Vmess".to_string(),
                udp: false,
                tcp: true,
                ..Default::default()
            },
        );
        proxies.insert(
            "none".to_string(),
            Proxy {
                name: "none".to_string(),
                proxy_type: "Direct".to_string(),
                udp: false,
                tcp: false,
                ..Default::default()
            },
        );

        let mut nodes = Vec::new();
        ProxyTree::push_entry(
            &mut nodes,
            "tcp_only",
            None,
            None,
            0,
            &proxies,
            &HashMap::new(),
            SortMode::None,
        );
        ProxyTree::push_entry(
            &mut nodes,
            "none",
            None,
            None,
            0,
            &proxies,
            &HashMap::new(),
            SortMode::None,
        );

        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].udp, false);
        assert_eq!(nodes[0].tcp, true);
        assert_eq!(nodes[1].udp, false);
        assert_eq!(nodes[1].tcp, false);
    }

    #[test]
    fn tcp_defaults_to_false_when_missing_from_json() {
        let json = r#"{"proxies":{"node":{"name":"node","type":"Vmess","udp":true}}}"#;
        let response: ProxiesResponse = serde_json::from_str(json).unwrap();
        let proxy = response.proxies.get("node").unwrap();
        assert_eq!(proxy.udp, true);
        assert_eq!(
            proxy.tcp, false,
            "tcp should default to false when missing from JSON"
        );
    }

    #[test]
    fn child_link_node_copies_udp_tcp_from_referenced_proxy() {
        let mut proxies = IndexMap::new();
        proxies.insert(
            "Parent".to_string(),
            Proxy {
                name: "Parent".to_string(),
                proxy_type: "Selector".to_string(),
                all: Some(vec!["Child".to_string()]),
                ..Default::default()
            },
        );
        proxies.insert(
            "Child".to_string(),
            Proxy {
                name: "Child".to_string(),
                proxy_type: "Vmess".to_string(),
                udp: true,
                tcp: false,
                ..Default::default()
            },
        );

        let mut nodes = Vec::new();
        let mut expanded = HashMap::new();
        expanded.insert("Parent".to_string(), true);
        ProxyTree::push_entry(
            &mut nodes,
            "Parent",
            None,
            None,
            0,
            &proxies,
            &expanded,
            SortMode::None,
        );

        let child = nodes.iter().find(|n| n.name == "Child").unwrap();
        assert_eq!(child.udp, true);
        assert_eq!(child.tcp, false);
    }

    fn load_singbox_fixture() -> IndexMap<String, Proxy> {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/apidata/sing-box/proxies.json"
        );
        let data = std::fs::read_to_string(path).expect("Failed to read sing-box fixture");
        let response: ProxiesResponse =
            serde_json::from_str(&data).expect("Failed to parse sing-box fixture");
        response.proxies
    }

    #[test]
    fn singbox_tree_builds_correct_top_level_groups() {
        let proxies = load_singbox_fixture();
        let tree = ProxyTree::build(ProxiesResponse {
            proxies: proxies.clone(),
        });
        let top_names: Vec<&str> = tree.nodes.iter().map(|n| n.name.as_str()).collect();
        assert!(top_names.contains(&"Entry"));
        assert!(top_names.contains(&"Select-hajimi"));
        assert!(top_names.contains(&"Select-manbo"));
        assert!(!top_names.contains(&"hidden-node"));
    }

    #[test]
    fn singbox_hidden_node_not_in_top_level() {
        let proxies = load_singbox_fixture();
        let hidden = proxies.get("hidden-node").expect("hidden-node missing");
        assert!(hidden.hidden);

        let tree = ProxyTree::build(ProxiesResponse {
            proxies: proxies.clone(),
        });
        let top_names: Vec<&str> = tree.nodes.iter().map(|n| n.name.as_str()).collect();
        assert!(!top_names.contains(&"hidden-node"));
    }

    #[test]
    fn singbox_tcp_defaults_false_when_absent() {
        let proxies = load_singbox_fixture();
        let jp = proxies.get("🇯🇵 日本-优化").expect("JP node missing");
        assert!(!jp.tcp);
        assert!(jp.udp);
    }

    #[test]
    fn singbox_tcp_true_when_explicit() {
        let proxies = load_singbox_fixture();
        let tcp_node = proxies.get("tcp-node").expect("tcp-node missing");
        assert!(tcp_node.tcp);
        assert!(!tcp_node.udp);
    }

    #[test]
    fn singbox_alive_false_node() {
        let proxies = load_singbox_fixture();
        let dead = proxies.get("dead-node").expect("dead-node missing");
        assert!(!dead.alive);
        assert_eq!(dead.proxy_type, "Shadowsocks");
    }

    #[test]
    fn singbox_trojan_node_type() {
        let proxies = load_singbox_fixture();
        let trojan = proxies.get("trojan-node").expect("trojan-node missing");
        assert_eq!(trojan.proxy_type, "Trojan");
        assert!(trojan.tcp);
        assert!(trojan.udp);
    }

    #[test]
    fn singbox_child_node_in_expanded_group() {
        let proxies = load_singbox_fixture();
        let mut tree = ProxyTree::build(ProxiesResponse {
            proxies: proxies.clone(),
        });
        tree.expand_all(&proxies);
        let tcp_child = tree.nodes.iter().find(|n| n.name == "tcp-node");
        assert!(tcp_child.is_some());
        assert_eq!(tcp_child.unwrap().tcp, true);
        assert_eq!(tcp_child.unwrap().udp, false);
    }

    #[test]
    fn mihomo_hidden_not_in_top_level() {
        let proxies = load_fixture();
        let hidden = proxies.get("hidden-proxy").expect("hidden-proxy missing");
        assert!(hidden.hidden);

        let tree = ProxyTree::build(ProxiesResponse {
            proxies: proxies.clone(),
        });
        let names: Vec<&str> = tree.nodes.iter().map(|n| n.name.as_str()).collect();
        assert!(!names.contains(&"hidden-proxy"));
    }

    #[test]
    fn mihomo_tcp_only_leaf() {
        let proxies = load_fixture();
        let tcp = proxies.get("tcp-only").expect("tcp-only missing");
        assert!(tcp.tcp);
        assert!(!tcp.udp);
    }

    #[test]
    fn mihomo_dead_proxy_alive_false() {
        let proxies = load_fixture();
        let dead = proxies.get("dead-proxy").expect("dead-proxy missing");
        assert!(!dead.alive);
        assert_eq!(dead.proxy_type, "Shadowsocks");
    }

    #[test]
    fn mihomo_loadbalance_group_type() {
        let proxies = load_fixture();
        let lb = proxies.get("LB-group").expect("LB-group missing");
        assert_eq!(lb.proxy_type, "LoadBalance");
        assert!(lb.all.as_ref().map(|a| a.len()).unwrap_or(0) > 0);
        assert!(lb.now.is_some());
    }

    #[test]
    fn mihomo_hysteria2_type_exists() {
        let proxies = load_fixture();
        let hy2 = proxies.get("日本JP-HY2").expect("JP HY2 missing");
        assert_eq!(hy2.proxy_type, "Hysteria2");
    }

    #[test]
    fn mihomo_now_chain_two_levels() {
        let proxies = load_fixture();
        let delay = resolve_delay("Entry", &proxies);
        assert!(delay.is_some(), "Entry should resolve delay via now chain");
        assert!(delay.unwrap() > 0);
    }

    #[test]
    fn mihomo_expand_all_tcp_child_visible() {
        let proxies = load_fixture();
        let mut tree = ProxyTree::build(ProxiesResponse {
            proxies: proxies.clone(),
        });
        tree.expand_all(&proxies);
        let tcp = tree.nodes.iter().find(|n| n.name == "tcp-only");
        assert!(tcp.is_some());
        assert_eq!(tcp.unwrap().tcp, true);
        assert_eq!(tcp.unwrap().udp, false);
    }
}
