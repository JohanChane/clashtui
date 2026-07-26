pub mod content;
pub mod handlers;
pub mod render;
pub mod tree;

use super::dev::*;
pub use content::Proxies;

newtype_tab!(ProxiesTab(Tab<Proxies>));

mod_agent!(
    keymap,
    crate::tui::binding::Scope::Proxies,
    Key,
    [
        ([KeyCode::Up], Key::MoveUp, "Move up"),
        ([KeyCode::Down], Key::MoveDown, "Move down"),
        ([KeyCode::Char('k')], Key::MoveUp, "Move up"),
        ([KeyCode::Char('j')], Key::MoveDown, "Move down"),
        ([KeyCode::Char('h')], Key::Parent, "Go to parent"),
        ([KeyCode::Char('l')], Key::Expand, "Expand/Jump to group"),
        ([KeyCode::Enter], Key::Select, "Select"),
        (
            [KeyCode::Char('g'), KeyCode::Char('g')],
            Key::GoTop,
            "Go to top"
        ),
        ([KeyCode::Char('G')], Key::GoBottom, "Go to bottom"),
        ([KeyCode::Char('/')], Key::Search, "Search/Filter"),
        (
            [KeyCode::Char('s'), KeyCode::Char('n')],
            Key::SortByName,
            "Sort by name"
        ),
        (
            [KeyCode::Char('s'), KeyCode::Char('d')],
            Key::SortByDelay,
            "Sort by delay"
        ),
        (
            [KeyCode::Char('s'), KeyCode::Char('r')],
            Key::ResetSort,
            "Reset sort"
        ),
        (
            [KeyCode::Char('S'), KeyCode::Char('n')],
            Key::GlobalSortByName,
            "Global sort by name"
        ),
        (
            [KeyCode::Char('S'), KeyCode::Char('d')],
            Key::GlobalSortByDelay,
            "Global sort by delay"
        ),
        (
            [KeyCode::Char('S'), KeyCode::Char('r')],
            Key::GlobalResetSort,
            "Global reset sort"
        ),
        (
            [KeyCode::Char('a'), KeyCode::Char('f')],
            Key::CollapseAll,
            "Collapse all"
        ),
        (
            [KeyCode::Char('a'), KeyCode::Char('e')],
            Key::ExpandAll,
            "Expand all"
        ),
        ([KeyCode::Char('t')], Key::TestDelay, "Test delay"),
        (
            [KeyCode::Char('a'), KeyCode::Char('t')],
            Key::TestAllDelay,
            "Test all delay"
        ),
        ([KeyCode::Char('r')], Key::Refresh, "Refresh"),
        ([KeyCode::Char('f')], Key::GroupSelect, "Group select"),
        ([KeyCode::Char('F')], Key::FzfFind, "Find proxy"),
    ]
);

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Key {
    MoveUp,
    MoveDown,
    Parent,
    Expand,
    Select,
    GoTop,
    GoBottom,
    CollapseAll,
    ExpandAll,
    SortByName,
    SortByDelay,
    ResetSort,
    GlobalSortByName,
    GlobalSortByDelay,
    GlobalResetSort,
    TestDelay,
    TestAllDelay,
    Refresh,
    Search,
    FzfFind,
    GroupSelect,
}

#[cfg(test)]
mod tests {
    use crate::tui::Key as TuiKey;
    use crate::tui::binding::DisplayBinding;
    use crate::tui::widget::chord::ChordHandler;
    use crossterm::event::KeyCode;

    use super::keymap;
    use super::{Key, Proxies};

    fn mk_key(code: KeyCode) -> TuiKey {
        TuiKey {
            code,
            shift: matches!(code, KeyCode::Char(c) if c.is_ascii_uppercase()),
            ctrl: false,
            alt: false,
            super_: false,
        }
    }

    fn make_display_bindings() -> Vec<DisplayBinding> {
        keymap::get().to_display()
    }

    #[test]
    fn chord_handler_a_initiates_chord_mode() {
        let a = mk_key(KeyCode::Char('a'));
        let bindings = make_display_bindings();
        let mut ch = ChordHandler::default();
        let mut dispatched: Vec<Vec<TuiKey>> = vec![];
        let consumed = ch.handle(&a, &bindings, &mut |seq| dispatched.push(seq.to_vec()));
        assert!(consumed, "chord should consume 'a'");
        assert!(dispatched.is_empty(), "no dispatch on first key");
        assert!(ch.is_active(), "chord should be active after 'a'");
        assert_eq!(ch.pressed.len(), 1);
        assert_eq!(
            ch.candidates.len(),
            3,
            "should have 3 candidates: CollapseAll, ExpandAll, TestAllDelay"
        );
    }

    #[test]
    fn expand_all_preserves_selected_node() {
        use crate::functions::restful::proxies::ProxiesResponse;
        use crate::tui::tab::proxies::tree::{NodeType, ProxyTree};
        use crate::tui::widget::tab::FutureSet;
        use ratatui::widgets::ListState;

        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/apidata/mihomo/proxies.json"
        );
        let data = std::fs::read_to_string(path).unwrap();
        let response: ProxiesResponse = serde_json::from_str(&data).unwrap();
        let proxies = response.proxies;

        let mut content = Proxies {
            tree: ProxyTree::build(ProxiesResponse {
                proxies: proxies.clone(),
            }),
            proxies: proxies.clone(),
            ..Default::default()
        };

        let mut state = ListState::default();
        let mut tasks: FutureSet<Proxies> = tokio::task::JoinSet::new();

        // Select a middle folder
        let folder_name = "Sl-hajimi";
        let folder_idx = content
            .tree
            .nodes
            .iter()
            .position(|n| n.node_type == NodeType::Folder && n.name == folder_name)
            .unwrap();
        state.select(Some(folder_idx));

        let num_before = content.tree.len();
        let saved = content.selection_key(&state).unwrap();

        // Expand all
        content.dispatch_key(Key::ExpandAll, &mut tasks, &mut state);

        let num_after = content.tree.len();
        assert!(
            num_after > num_before,
            "expand_all should increase the tree size"
        );

        let new_idx = state.selected().unwrap();
        let node = content.tree.node_at(new_idx).unwrap();
        assert_eq!(
            (node.name.as_str(), &node.parent, &node.node_type),
            (saved.0.as_str(), &saved.1, &saved.2),
            "ExpandAll should preserve the selected node identity"
        );
        assert_eq!(node.node_type, NodeType::Folder);
        assert_eq!(node.name, folder_name);
    }

    #[test]
    fn chord_af_still_dispatches_collapse_all() {
        let a = mk_key(KeyCode::Char('a'));
        let f = mk_key(KeyCode::Char('f'));
        let bindings = make_display_bindings();

        assert!(
            keymap::get().find_by_seq(&[f]).is_some(),
            "f alone should be a single-key shortcut"
        );
        assert!(
            keymap::get()
                .find_by_seq(&[a, f])
                .is_some_and(|k| matches!(k, Key::CollapseAll)),
            "af chord should dispatch CollapseAll"
        );

        let mut ch = ChordHandler::default();
        let mut dispatched: Vec<Vec<TuiKey>> = vec![];
        ch.handle(&a, &bindings, &mut |seq| dispatched.push(seq.to_vec()));
        let consumed = ch.handle(&f, &bindings, &mut |seq| dispatched.push(seq.to_vec()));
        assert!(consumed, "af should be consumed by chord handler");
        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0].len(), 2);
        assert!(!ch.is_active());
    }

    #[test]
    fn group_select_on_folder_collects_siblings() {
        use crate::functions::restful::proxies::ProxiesResponse;
        use crate::tui::tab::proxies::tree::{NodeType, ProxyTree};

        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/apidata/mihomo/proxies.json"
        );
        let data = std::fs::read_to_string(path).unwrap();
        let response: ProxiesResponse = serde_json::from_str(&data).unwrap();
        let proxies = response.proxies;

        let content = Proxies {
            tree: ProxyTree::build(ProxiesResponse {
                proxies: proxies.clone(),
            }),
            proxies: proxies.clone(),
            ..Default::default()
        };

        // Sl-pvd0 is a top-level Folder (depth 0, parent=None)
        let folder_idx = content
            .tree
            .nodes
            .iter()
            .position(|n| n.node_type == NodeType::Folder && n.name == "Sl-hajimi")
            .unwrap();
        let parent = content
            .tree
            .node_at(folder_idx)
            .and_then(|n| n.parent.clone());

        let siblings: Vec<&str> = content
            .tree
            .nodes
            .iter()
            .filter(|n| n.parent == parent)
            .map(|n| n.name.as_str())
            .collect();

        // All top-level (parent=None) Folder nodes should be siblings
        assert!(
            siblings.contains(&"Sl-hajimi"),
            "Folder itself should be in siblings"
        );
        assert!(siblings.contains(&"Entry"), "Entry is a top-level sibling");
        assert!(
            !siblings.contains(&"vmess-node001"),
            "vmess-ipdktc33 is a child, not a sibling"
        );
    }

    #[test]
    fn group_select_on_child_collects_siblings_in_same_group() {
        use crate::functions::restful::proxies::ProxiesResponse;
        use crate::tui::tab::proxies::tree::{NodeType, ProxyTree};
        use crate::tui::widget::tab::FutureSet;
        use ratatui::widgets::ListState;

        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/apidata/mihomo/proxies.json"
        );
        let data = std::fs::read_to_string(path).unwrap();
        let response: ProxiesResponse = serde_json::from_str(&data).unwrap();
        let proxies = response.proxies;

        let mut content = Proxies {
            tree: ProxyTree::build(ProxiesResponse {
                proxies: proxies.clone(),
            }),
            proxies: proxies.clone(),
            ..Default::default()
        };

        let mut state = ListState::default();
        let mut tasks: FutureSet<Proxies> = tokio::task::JoinSet::new();

        // Expand Entry to reveal children
        let entry_idx = content
            .tree
            .nodes
            .iter()
            .position(|n| n.node_type == NodeType::Folder && n.name == "Entry")
            .unwrap();
        state.select(Some(entry_idx));
        content.dispatch_key(Key::Expand, &mut tasks, &mut state);

        // Sl-pvd0 is a child (Link) of Entry
        let child_idx = content
            .tree
            .nodes
            .iter()
            .position(|n| n.name == "Sl-hajimi" && n.parent.as_deref() == Some("Entry"))
            .unwrap();
        let parent = content
            .tree
            .node_at(child_idx)
            .and_then(|n| n.parent.clone());

        let siblings: Vec<&str> = content
            .tree
            .nodes
            .iter()
            .filter(|n| n.parent == parent)
            .map(|n| n.name.as_str())
            .collect();

        assert!(
            siblings.contains(&"Sl-hajimi"),
            "Sl-pvd0 itself should be in siblings"
        );
        assert!(
            siblings.contains(&"At-hajimi"),
            "At-pvd0 is a sibling under Entry"
        );
        assert!(
            siblings.contains(&"Sl-manbo"),
            "FltAt-pvd0 is a sibling under Entry"
        );
        assert!(
            !siblings.contains(&"Entry"),
            "Entry is the parent, not a sibling"
        );
        // Expand At-manbo (a top-level Folder) and verify it has children
        let at_manbo_idx = content
            .tree
            .nodes
            .iter()
            .position(|n| n.node_type == NodeType::Folder && n.name == "At-manbo")
            .unwrap();
        state.select(Some(at_manbo_idx));
        content.dispatch_key(Key::Expand, &mut tasks, &mut state);
        let at_manbo_children: Vec<&str> = content
            .tree
            .nodes
            .iter()
            .filter(|n| n.parent.as_deref() == Some("At-manbo"))
            .map(|n| n.name.as_str())
            .collect();
        assert!(
            !at_manbo_children.is_empty(),
            "At-manbo should have children after expand"
        );
    }
}
