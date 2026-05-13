pub mod content;
pub mod handlers;
pub mod render;
pub mod tree;

use super::dev::*;
pub use content::Proxies;

newtype_tab!(ProxiesTab(Tab<Proxies>));

mod_agent!(
    Key,
    [
        ([KeyCode::Up], Key::MoveUp, "Move up"),
        ([KeyCode::Down], Key::MoveDown, "Move down"),
        ([KeyCode::Char('k')], Key::MoveUp, "Move up"),
        ([KeyCode::Char('j')], Key::MoveDown, "Move down"),
        ([KeyCode::Char('h')], Key::Parent, "Go to parent"),
        ([KeyCode::Char('l')], Key::Expand, "Expand"),
        ([KeyCode::Enter], Key::Select, "Select"),
        ([KeyCode::Char('g'), KeyCode::Char('g')], Key::GoTop, "Go to top"),
        ([KeyCode::Char('G')], Key::GoBottom, "Go to bottom"),
        ([KeyCode::Char('/')], Key::Search, "Search/Filter"),
        ([KeyCode::Char('s'), KeyCode::Char('n')], Key::SortByName, "Sort by name"),
        ([KeyCode::Char('s'), KeyCode::Char('d')], Key::SortByDelay, "Sort by delay"),
        ([KeyCode::Char('s'), KeyCode::Char('r')], Key::ResetSort, "Reset sort"),
        ([KeyCode::Char('S'), KeyCode::Char('n')], Key::GlobalSortByName, "Global sort by name"),
        ([KeyCode::Char('S'), KeyCode::Char('d')], Key::GlobalSortByDelay, "Global sort by delay"),
        ([KeyCode::Char('S'), KeyCode::Char('r')], Key::GlobalResetSort, "Global reset sort"),
        ([KeyCode::Char('a'), KeyCode::Char('f')], Key::CollapseAll, "Collapse all"),
        ([KeyCode::Char('a'), KeyCode::Char('e')], Key::ExpandAll, "Expand all"),
        ([KeyCode::Char('t')], Key::TestDelay, "Test delay"),
        ([KeyCode::Char('a'), KeyCode::Char('t')], Key::TestAllDelay, "Test all delay"),
        ([KeyCode::Char('r')], Key::Refresh, "Refresh"),
        ([KeyCode::Char('f')], Key::FzfFind, "Find proxy"),
    ]
);

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
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
}

impl TryFrom<&crate::tui::Key> for Key {
    type Error = ();

    fn try_from(ev: &crate::tui::Key) -> Result<Self, Self::Error> {
        let agent = agent();
        if !agent.is_empty() {
            return agent.get(ev).map(|act| *act).ok_or(());
        }
        Err(())
    }
}


#[cfg(test)]
mod tests {
    use crate::tui::Key as TuiKey;
    use crate::tui::widget::{chord::ChordHandler, tab::KeyCombo};
    use crossterm::event::KeyCode;

    use super::{Key, Proxies, Tab, agent};
    use super::all_shortcuts;

    fn mk_key(code: KeyCode) -> TuiKey {
        TuiKey {
            code,
            shift: matches!(code, KeyCode::Char(c) if c.is_ascii_uppercase()),
            ctrl: false,
            alt: false,
            super_: false,
        }
    }

    fn make_shortcuts() -> Vec<(KeyCombo, &'static str)> {
        all_shortcuts().iter().map(|(c, _, d)| (c.clone(), *d)).collect()
    }

    #[test]
    fn all_shortcuts_contains_chords() {
        let descs: Vec<&str> = all_shortcuts().iter()
            .filter(|(_, k, _)| matches!(k, Key::CollapseAll | Key::ExpandAll))
            .map(|(_, _, d)| *d)
            .collect();
        assert_eq!(descs, vec!["Collapse all", "Expand all"]);
    }

    #[test]
    fn single_key_shortcuts_in_agent() {
        let a = agent();
        assert!(a.contains_key(&mk_key(KeyCode::Char('j'))));
    }

    #[test]
    fn try_from_uses_agent() {
        let kev = mk_key(KeyCode::Char('j'));
        let key = Key::try_from(&kev);
        assert!(matches!(key, Ok(Key::MoveDown)));
    }

    #[test]
    fn chords_not_in_try_from() {
        let kev = mk_key(KeyCode::Char('a'));
        let key = Key::try_from(&kev);
        assert!(key.is_err());
    }

    #[test]
    fn dispatch_shortcut_matches_chord() {
        let seq: Vec<TuiKey> = vec![mk_key(KeyCode::Char('a')), mk_key(KeyCode::Char('f'))];
        let found = all_shortcuts().iter().any(|(combo, _, _)| &**combo == seq);
        assert!(found, "dispatch_shortcut should find (a,f) chord");
    }

    #[test]
    fn chord_handler_a_initiates_chord_mode() {
        let a = mk_key(KeyCode::Char('a'));
        let shortcuts = make_shortcuts();
        let mut ch = ChordHandler::default();
        let mut dispatched: Vec<Vec<TuiKey>> = vec![];
        let consumed = ch.handle(&a, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        assert!(consumed, "chord should consume 'a'");
        assert!(dispatched.is_empty(), "no dispatch on first key");
        assert!(ch.is_active(), "chord should be active after 'a'");
        assert_eq!(ch.pressed.len(), 1);
        assert_eq!(ch.candidates.len(), 3, "should have 3 candidates: CollapseAll, ExpandAll, TestAllDelay");
    }

    #[test]
    fn chord_handler_Sn_dispatches_global_sort_by_name() {
        let s = mk_key(KeyCode::Char('S'));
        let n = mk_key(KeyCode::Char('n'));
        let shortcuts = make_shortcuts();
        let mut ch = ChordHandler::default();
        let mut dispatched: Vec<Vec<TuiKey>> = vec![];
        ch.handle(&s, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        let consumed = ch.handle(&n, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        assert!(consumed);
        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0].len(), 2);
        assert!(!ch.is_active());
    }

    #[test]
    fn chord_handler_af_dispatches_collapse_all() {
        let a = mk_key(KeyCode::Char('a'));
        let f = mk_key(KeyCode::Char('f'));
        let shortcuts = make_shortcuts();
        let mut ch = ChordHandler::default();
        let mut dispatched: Vec<Vec<TuiKey>> = vec![];
        ch.handle(&a, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        let consumed = ch.handle(&f, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        assert!(consumed);
        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0].len(), 2);
        assert!(!ch.is_active(), "chord should reset after dispatch");
    }

    #[test]
    fn chord_handler_ae_dispatches_expand_all() {
        let a = mk_key(KeyCode::Char('a'));
        let e = mk_key(KeyCode::Char('e'));
        let shortcuts = make_shortcuts();
        let mut ch = ChordHandler::default();
        let mut dispatched: Vec<Vec<TuiKey>> = vec![];
        ch.handle(&a, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        let consumed = ch.handle(&e, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        assert!(consumed);
        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0].len(), 2);
        assert!(!ch.is_active());
    }

    #[test]
    fn s_no_longer_a_single_key_shortcut() {
        let kev = mk_key(KeyCode::Char('s'));
        assert!(Key::try_from(&kev).is_err(), "'s' should no longer be a single-key shortcut");
    }

    #[test]
    fn tab_shortcuts_returns_chord_entries() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();
        let tab = Tab::<Proxies>::default();
        let shortcuts = tab.shortcuts();
        assert!(!shortcuts.is_empty(), "Tab<Proxies>::shortcuts() should not be empty");
        let has_chord = shortcuts.iter().any(|(combo, _)| {
            combo.len() > 1 && combo[0] == mk_key(KeyCode::Char('a'))
        });
        assert!(has_chord, "Tab<Proxies>::shortcuts() should contain 'a' chord entries");
    }

    #[test]
    fn chord_handler_a_single_key_still_works() {
        let j = mk_key(KeyCode::Char('j'));
        let shortcuts = make_shortcuts();
        let mut ch = ChordHandler::default();
        let mut dispatched: Vec<Vec<TuiKey>> = vec![];
        let consumed = ch.handle(&j, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        assert!(consumed);
        assert_eq!(dispatched.len(), 1);
        assert!(!ch.is_active());
    }

    #[test]
    fn chord_handler_gg_dispatches_go_top() {
        let g = mk_key(KeyCode::Char('g'));
        let shortcuts = make_shortcuts();
        let mut ch = ChordHandler::default();
        let mut dispatched: Vec<Vec<TuiKey>> = vec![];
        ch.handle(&g, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        let consumed = ch.handle(&g, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        assert!(consumed);
        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0].len(), 2);
        assert!(!ch.is_active());
    }

    #[test]
    fn single_key_G_in_agent() {
        let kev = mk_key(KeyCode::Char('G'));
        let key = Key::try_from(&kev);
        assert!(matches!(key, Ok(Key::GoBottom)), "G should map to GoBottom");
    }

    #[test]
    fn single_key_slash_in_agent() {
        let kev = mk_key(KeyCode::Char('/'));
        let key = Key::try_from(&kev);
        assert!(matches!(key, Ok(Key::Search)), "/ should map to Search");
    }

    #[test]
    fn s_initiates_chord_mode() {
        let s_lower = mk_key(KeyCode::Char('s'));
        let shortcuts = make_shortcuts();
        let mut ch = ChordHandler::default();
        let mut dispatched: Vec<Vec<TuiKey>> = vec![];
        let consumed = ch.handle(&s_lower, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        assert!(consumed, "s should initiate chord mode");
        assert!(dispatched.is_empty(), "no dispatch on first key");
        assert!(ch.is_active());
        assert_eq!(ch.candidates.len(), 3, "s should have 3 candidates: group sort by name/delay/reset");
    }

    #[test]
    fn sn_dispatches_group_sort_by_name() {
        let s = mk_key(KeyCode::Char('s'));
        let n = mk_key(KeyCode::Char('n'));
        let shortcuts = make_shortcuts();
        let mut ch = ChordHandler::default();
        let mut dispatched: Vec<Vec<TuiKey>> = vec![];
        ch.handle(&s, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        let consumed = ch.handle(&n, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        assert!(consumed);
        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0].len(), 2);
        assert!(!ch.is_active());
    }

    #[test]
    fn sd_dispatches_group_sort_by_delay() {
        let s = mk_key(KeyCode::Char('s'));
        let d = mk_key(KeyCode::Char('d'));
        let shortcuts = make_shortcuts();
        let mut ch = ChordHandler::default();
        let mut dispatched: Vec<Vec<TuiKey>> = vec![];
        ch.handle(&s, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        let consumed = ch.handle(&d, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        assert!(consumed);
        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0].len(), 2);
        assert!(!ch.is_active());
    }

    #[test]
    fn sr_dispatches_group_reset_sort() {
        let s = mk_key(KeyCode::Char('s'));
        let r = mk_key(KeyCode::Char('r'));
        let shortcuts = make_shortcuts();
        let mut ch = ChordHandler::default();
        let mut dispatched: Vec<Vec<TuiKey>> = vec![];
        ch.handle(&s, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        let consumed = ch.handle(&r, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        assert!(consumed);
        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0].len(), 2);
        assert!(!ch.is_active());
    }

    #[test]
    fn S_initiates_chord_mode() {
        let s_upper = mk_key(KeyCode::Char('S'));
        let shortcuts = make_shortcuts();
        let mut ch = ChordHandler::default();
        let mut dispatched: Vec<Vec<TuiKey>> = vec![];
        let consumed = ch.handle(&s_upper, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        assert!(consumed, "S should initiate chord mode");
        assert!(dispatched.is_empty(), "no dispatch on first key");
        assert!(ch.is_active());
        assert_eq!(ch.candidates.len(), 3, "S should have 3 candidates: global sort by name/delay/reset");
    }

    #[test]
    fn Sd_dispatches_global_sort_by_delay() {
        let s_upper = mk_key(KeyCode::Char('S'));
        let d = mk_key(KeyCode::Char('d'));
        let shortcuts = make_shortcuts();
        let mut ch = ChordHandler::default();
        let mut dispatched: Vec<Vec<TuiKey>> = vec![];
        ch.handle(&s_upper, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        let consumed = ch.handle(&d, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        assert!(consumed);
        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0].len(), 2);
        assert!(!ch.is_active());
    }

    #[test]
    fn Sr_dispatches_global_reset_sort() {
        let s_upper = mk_key(KeyCode::Char('S'));
        let r = mk_key(KeyCode::Char('r'));
        let shortcuts = make_shortcuts();
        let mut ch = ChordHandler::default();
        let mut dispatched: Vec<Vec<TuiKey>> = vec![];
        ch.handle(&s_upper, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        let consumed = ch.handle(&r, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        assert!(consumed);
        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0].len(), 2);
        assert!(!ch.is_active());
    }

    #[test]
    fn expand_all_preserves_selected_node() {
        use crate::functions::restful::proxies::ProxiesResponse;
        use crate::tui::tab::proxies::tree::{NodeType, ProxyTree};
        use crate::tui::widget::tab::FutureSet;
        use ratatui::widgets::ListState;

        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/tui/tab/proxies/tests/fixtures/proxies.json"
        );
        let data = std::fs::read_to_string(path).unwrap();
        let response: ProxiesResponse = serde_json::from_str(&data).unwrap();
        let proxies = response.proxies;

        let mut content = Proxies {
            tree: ProxyTree::build(ProxiesResponse { proxies: proxies.clone() }),
            proxies: proxies.clone(),
            ..Default::default()
        };

        let mut state = ListState::default();
        let mut tasks: FutureSet<Proxies> = tokio::task::JoinSet::new();

        // Select a middle folder
        let folder_name = "Sl-pvd0";
        let folder_idx = content.tree.nodes.iter()
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
}
