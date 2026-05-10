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
        ([KeyCode::Up], Key::MoveUp, ""),
        ([KeyCode::Down], Key::MoveDown, ""),
        ([KeyCode::Char('k')], Key::MoveUp, ""),
        ([KeyCode::Char('j')], Key::MoveDown, ""),
        ([KeyCode::Char('h')], Key::Parent, ""),
        ([KeyCode::Char('l')], Key::Expand, ""),
        ([KeyCode::Enter], Key::Select, ""),
        ([KeyCode::Char('s'), KeyCode::Char('n')], Key::SortByName, "Sort by name"),
        ([KeyCode::Char('s'), KeyCode::Char('d')], Key::SortByDelay, "Sort by delay"),
        ([KeyCode::Char('s'), KeyCode::Char('r')], Key::ResetSort, "Reset sort"),
        ([KeyCode::Char('a'), KeyCode::Char('f')], Key::CollapseAll, "Collapse all"),
        ([KeyCode::Char('a'), KeyCode::Char('e')], Key::ExpandAll, "Expand all"),
        ([KeyCode::Char('t')], Key::TestDelay, "Test delay"),
        ([KeyCode::Char('a'), KeyCode::Char('t')], Key::TestAllDelay, "Test all delay"),
        ([KeyCode::Char('r')], Key::Refresh, "Refresh"),
        ([KeyCode::Char('f')], Key::FzfFind, "Fuzzy find proxy"),
    ]
);

#[derive(Clone, Copy)]
pub enum Key {
    MoveUp,
    MoveDown,
    Parent,
    Expand,
    Select,
    CollapseAll,
    ExpandAll,
    SortByName,
    SortByDelay,
    ResetSort,
    TestDelay,
    TestAllDelay,
    Refresh,
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
        TuiKey { code, shift: false, ctrl: false, alt: false, super_: false }
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
    fn chord_handler_sn_dispatches_sort_by_name() {
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
}
