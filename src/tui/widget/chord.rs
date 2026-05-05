use crate::tui::Key;
use crossterm::event::KeyCode;

use super::tab::KeyCombo;

#[derive(Default)]
pub struct ChordHandler {
    pub pressed: Vec<Key>,
    pub candidates: Vec<(KeyCombo, &'static str)>,
}

impl ChordHandler {
    pub fn is_active(&self) -> bool {
        !self.pressed.is_empty()
    }

    pub fn handle(
        &mut self,
        kv: &Key,
        shortcuts: &[(KeyCombo, &'static str)],
        dispatch: &mut dyn FnMut(&[Key]),
    ) -> bool {
        if self.is_active() {
            self.continue_(kv, dispatch)
        } else {
            self.check_init(kv, shortcuts, dispatch)
        }
    }

    fn reset(&mut self) {
        self.pressed.clear();
        self.candidates.clear();
    }

    fn continue_(&mut self, kv: &Key, dispatch: &mut dyn FnMut(&[Key])) -> bool {
        if kv.code == KeyCode::Esc && !kv.ctrl && !kv.alt && !kv.super_ {
            self.reset();
            return true;
        }

        let idx = self.pressed.len();
        self.pressed.push(*kv);
        self.candidates
            .retain(|(seq, _)| idx < seq.len() && seq[idx] == *kv);

        match self.candidates.len() {
            0 => {
                self.reset();
                true
            }
            1 => {
                let seq = self.candidates[0].0.clone();
                self.reset();
                dispatch(&seq);
                true
            }
            _ => {
                if let Some((exact, _)) = self
                    .candidates
                    .iter()
                    .find(|(s, _)| s.len() == self.pressed.len())
                {
                    let seq = exact.clone();
                    self.reset();
                    dispatch(&seq);
                }
                true
            }
        }
    }

    fn check_init(
        &mut self,
        kv: &Key,
        shortcuts: &[(KeyCombo, &'static str)],
        dispatch: &mut dyn FnMut(&[Key]),
    ) -> bool {
        for (seq, _) in shortcuts {
            if seq.len() == 1 && seq[0] == *kv {
                dispatch(&[*kv]);
                return true;
            }
        }

        let candidates: Vec<(KeyCombo, &str)> = shortcuts
            .iter()
            .filter(|(seq, _)| seq.len() > 1 && seq[0] == *kv)
            .cloned()
            .collect();

        if !candidates.is_empty() {
            self.pressed = vec![*kv];
            self.candidates = candidates;
            return true;
        }

        false
    }
}

pub fn key_event_to_str(k: &Key) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    write!(s, "{k}").unwrap();
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_key(code: KeyCode) -> Key {
        Key { code, shift: false, ctrl: false, alt: false, super_: false }
    }

    fn mk_key_mod(code: KeyCode, ctrl: bool) -> Key {
        Key { code, ctrl, shift: false, alt: false, super_: false }
    }

    fn make_shortcuts(data: &[(&[KeyCode], &'static str)]) -> Vec<(KeyCombo, &'static str)> {
        data.iter()
            .map(|(codes, desc)| {
                (
                    KeyCombo(codes.iter().map(|c| mk_key(*c)).collect()),
                    *desc,
                )
            })
            .collect()
    }

    #[test]
    fn chord_init_single_key_dispatches() {
        let g = mk_key(KeyCode::Char('g'));
        let shortcuts = make_shortcuts(&[(&[KeyCode::Char('g')], "Action")]);
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<Key>> = vec![];

        let consumed =
            handler.handle(&g, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));

        assert!(consumed);
        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0].len(), 1);
        assert_eq!(dispatched[0][0], g);
        assert!(!handler.is_active());
    }

    #[test]
    fn chord_init_multi_key_enters_chord_mode() {
        let g = mk_key(KeyCode::Char('g'));
        let shortcuts = make_shortcuts(&[(&[KeyCode::Char('g'), KeyCode::Char('g')], "GoTop")]);
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<Key>> = vec![];

        let consumed =
            handler.handle(&g, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));

        assert!(consumed);
        assert!(dispatched.is_empty());
        assert!(handler.is_active());
        assert_eq!(handler.pressed.len(), 1);
        assert_eq!(handler.candidates.len(), 1);
    }

    #[test]
    fn chord_continue_matching_dispatches() {
        let g = mk_key(KeyCode::Char('g'));
        let shortcuts = make_shortcuts(&[(&[KeyCode::Char('g'), KeyCode::Char('g')], "GoTop")]);
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<Key>> = vec![];

        handler.handle(&g, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        let consumed = handler.handle(&g, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));

        assert!(consumed);
        assert_eq!(dispatched.len(), 1);
        assert!(!handler.is_active());
    }

    #[test]
    fn chord_continue_non_matching_cancels_and_consumes() {
        let g = mk_key(KeyCode::Char('g'));
        let x = mk_key(KeyCode::Char('x'));
        let shortcuts = make_shortcuts(&[(&[KeyCode::Char('g'), KeyCode::Char('g')], "GoTop")]);
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<Key>> = vec![];

        handler.handle(&g, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        let consumed = handler.handle(&x, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));

        assert!(consumed);
        assert!(dispatched.is_empty());
        assert!(!handler.is_active());
    }

    #[test]
    fn chord_esc_cancels_and_consumes() {
        let g = mk_key(KeyCode::Char('g'));
        let esc = mk_key(KeyCode::Esc);
        let shortcuts = make_shortcuts(&[(&[KeyCode::Char('g'), KeyCode::Char('g')], "GoTop")]);
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<Key>> = vec![];

        handler.handle(&g, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        let consumed =
            handler.handle(&esc, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));

        assert!(consumed);
        assert!(dispatched.is_empty());
        assert!(!handler.is_active());
    }

    #[test]
    fn single_key_shortcut_takes_priority_over_chord_prefix() {
        let d = mk_key(KeyCode::Char('d'));
        let shortcuts = make_shortcuts(&[
            (&[KeyCode::Char('d')], "Delete"),
            (&[KeyCode::Char('d'), KeyCode::Char('d')], "DeleteAll"),
        ]);
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<Key>> = vec![];

        let consumed =
            handler.handle(&d, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));

        assert!(consumed);
        assert_eq!(dispatched.len(), 1);
        assert!(!handler.is_active());
    }

    #[test]
    fn exact_match_dispatches_among_multiple_candidates() {
        let g = mk_key(KeyCode::Char('g'));
        let e = mk_key(KeyCode::Char('e'));
        let shortcuts = make_shortcuts(&[
            (&[KeyCode::Char('g'), KeyCode::Char('g')], "GoTop"),
            (&[KeyCode::Char('g'), KeyCode::Char('e')], "GoEnd"),
        ]);
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<Key>> = vec![];

        handler.handle(&g, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        let consumed = handler.handle(&e, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));

        assert!(consumed);
        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0].len(), 2);
        assert!(!handler.is_active());
    }

    #[test]
    fn ctrl_c_does_not_cancel_chord() {
        let g = mk_key(KeyCode::Char('g'));
        let cc = mk_key_mod(KeyCode::Char('c'), true);
        let shortcuts = make_shortcuts(&[(&[KeyCode::Char('g'), KeyCode::Char('g')], "GoTop")]);
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<Key>> = vec![];

        handler.handle(&g, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        let consumed = handler.handle(&cc, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));

        assert!(consumed);
        assert!(dispatched.is_empty());
        assert!(!handler.is_active(), "Ctrl-C as a chord mismatch should cancel chord");
    }

    #[test]
    fn ctrl_c_keybinding_dispatches_on_initial_press() {
        let cc = mk_key_mod(KeyCode::Char('c'), true);
        let shortcuts = vec![
            (KeyCombo(vec![cc]), "Close"),
        ];
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<Key>> = vec![];

        let consumed = handler.handle(&cc, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));

        assert!(consumed);
        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0][0], cc);
        assert!(!handler.is_active());
    }
}
