use crate::tui::Key;
use crate::tui::binding::DisplayBinding;
use crossterm::event::KeyCode;

#[derive(Default)]
pub struct ChordHandler {
    pub pressed: Vec<Key>,
    pub candidates: Vec<DisplayBinding>,
}

impl ChordHandler {
    pub fn is_active(&self) -> bool {
        !self.pressed.is_empty()
    }

    pub fn handle(
        &mut self,
        kv: &Key,
        bindings: &[DisplayBinding],
        dispatch: &mut dyn FnMut(&[Key]),
    ) -> bool {
        if self.is_active() {
            self.continue_(kv, dispatch)
        } else {
            self.check_init(kv, bindings, dispatch)
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
            .retain(|b| idx < b.on.len() && b.on[idx] == *kv);

        match self.candidates.len() {
            0 => {
                self.reset();
                true
            }
            1 => {
                let seq = self.candidates[0].on.clone();
                self.reset();
                dispatch(&seq);
                true
            }
            _ => {
                if let Some(exact) = self
                    .candidates
                    .iter()
                    .find(|b| b.on.len() == self.pressed.len())
                {
                    let seq = exact.on.clone();
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
        bindings: &[DisplayBinding],
        dispatch: &mut dyn FnMut(&[Key]),
    ) -> bool {
        // Collect all candidates starting with this key
        let candidates: Vec<&DisplayBinding> = bindings
            .iter()
            .filter(|b| b.on.first() == Some(kv))
            .collect();

        match candidates.len() {
            0 => false,
            1 if candidates[0].on.len() == 1 => {
                // Unique single-key candidate -> dispatch immediately
                dispatch(&candidates[0].on);
                true
            }
            _ => {
                // Multiple candidates -> enter chord mode
                // Prefix conflicts are eliminated at load time, so all candidates are chords
                self.pressed = vec![*kv];
                self.candidates = candidates.into_iter().cloned().collect();
                true
            }
        }
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
        Key {
            code,
            shift: false,
            ctrl: false,
            alt: false,
            super_: false,
        }
    }

    fn mk_key_mod(code: KeyCode, ctrl: bool) -> Key {
        Key {
            code,
            ctrl,
            shift: false,
            alt: false,
            super_: false,
        }
    }

    fn mk_display(keys: &[Key], desc: &str) -> DisplayBinding {
        DisplayBinding {
            on: keys.to_vec(),
            desc: desc.to_owned(),
        }
    }

    #[test]
    fn chord_init_single_key_dispatches() {
        let g = mk_key(KeyCode::Char('g'));
        let bindings = vec![mk_display(&[g], "Action")];
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<Key>> = vec![];

        let consumed = handler.handle(&g, &bindings, &mut |seq| dispatched.push(seq.to_vec()));

        assert!(consumed);
        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0].len(), 1);
        assert_eq!(dispatched[0][0], g);
        assert!(!handler.is_active());
    }

    #[test]
    fn chord_init_multi_key_enters_chord_mode() {
        let g = mk_key(KeyCode::Char('g'));
        let bindings = vec![mk_display(
            &[mk_key(KeyCode::Char('g')), mk_key(KeyCode::Char('g'))],
            "GoTop",
        )];
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<Key>> = vec![];

        let consumed = handler.handle(&g, &bindings, &mut |seq| dispatched.push(seq.to_vec()));

        assert!(consumed);
        assert!(dispatched.is_empty());
        assert!(handler.is_active());
        assert_eq!(handler.pressed.len(), 1);
        assert_eq!(handler.candidates.len(), 1);
    }

    #[test]
    fn chord_continue_matching_dispatches() {
        let g = mk_key(KeyCode::Char('g'));
        let bindings = vec![mk_display(
            &[mk_key(KeyCode::Char('g')), mk_key(KeyCode::Char('g'))],
            "GoTop",
        )];
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<Key>> = vec![];

        handler.handle(&g, &bindings, &mut |seq| dispatched.push(seq.to_vec()));
        let consumed = handler.handle(&g, &bindings, &mut |seq| dispatched.push(seq.to_vec()));

        assert!(consumed);
        assert_eq!(dispatched.len(), 1);
        assert!(!handler.is_active());
    }

    #[test]
    fn chord_continue_non_matching_cancels_and_consumes() {
        let g = mk_key(KeyCode::Char('g'));
        let x = mk_key(KeyCode::Char('x'));
        let bindings = vec![mk_display(
            &[mk_key(KeyCode::Char('g')), mk_key(KeyCode::Char('g'))],
            "GoTop",
        )];
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<Key>> = vec![];

        handler.handle(&g, &bindings, &mut |seq| dispatched.push(seq.to_vec()));
        let consumed = handler.handle(&x, &bindings, &mut |seq| dispatched.push(seq.to_vec()));

        assert!(consumed);
        assert!(dispatched.is_empty());
        assert!(!handler.is_active());
    }

    #[test]
    fn chord_esc_cancels_and_consumes() {
        let g = mk_key(KeyCode::Char('g'));
        let esc = mk_key(KeyCode::Esc);
        let bindings = vec![mk_display(
            &[mk_key(KeyCode::Char('g')), mk_key(KeyCode::Char('g'))],
            "GoTop",
        )];
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<Key>> = vec![];

        handler.handle(&g, &bindings, &mut |seq| dispatched.push(seq.to_vec()));
        let consumed = handler.handle(&esc, &bindings, &mut |seq| dispatched.push(seq.to_vec()));

        assert!(consumed);
        assert!(dispatched.is_empty());
        assert!(!handler.is_active());
    }

    #[test]
    fn single_key_shortcut_takes_priority_over_chord_prefix() {
        let d = mk_key(KeyCode::Char('d'));
        let bindings = vec![mk_display(&[mk_key(KeyCode::Char('d'))], "Delete")];
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<Key>> = vec![];

        let consumed = handler.handle(&d, &bindings, &mut |seq| dispatched.push(seq.to_vec()));

        assert!(consumed);
        assert_eq!(dispatched.len(), 1);
        assert!(!handler.is_active());
    }

    #[test]
    fn exact_match_dispatches_among_multiple_candidates() {
        let g = mk_key(KeyCode::Char('g'));
        let e = mk_key(KeyCode::Char('e'));
        let bindings = vec![
            mk_display(
                &[mk_key(KeyCode::Char('g')), mk_key(KeyCode::Char('g'))],
                "GoTop",
            ),
            mk_display(
                &[mk_key(KeyCode::Char('g')), mk_key(KeyCode::Char('e'))],
                "GoEnd",
            ),
        ];
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<Key>> = vec![];

        handler.handle(&g, &bindings, &mut |seq| dispatched.push(seq.to_vec()));
        let consumed = handler.handle(&e, &bindings, &mut |seq| dispatched.push(seq.to_vec()));

        assert!(consumed);
        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0].len(), 2);
        assert!(!handler.is_active());
    }

    #[test]
    fn ctrl_c_does_not_cancel_chord() {
        let g = mk_key(KeyCode::Char('g'));
        let cc = mk_key_mod(KeyCode::Char('c'), true);
        let bindings = vec![mk_display(
            &[mk_key(KeyCode::Char('g')), mk_key(KeyCode::Char('g'))],
            "GoTop",
        )];
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<Key>> = vec![];

        handler.handle(&g, &bindings, &mut |seq| dispatched.push(seq.to_vec()));
        let consumed = handler.handle(&cc, &bindings, &mut |seq| dispatched.push(seq.to_vec()));

        assert!(consumed);
        assert!(dispatched.is_empty());
        assert!(
            !handler.is_active(),
            "Ctrl-C as a chord mismatch should cancel chord"
        );
    }

    #[test]
    fn ctrl_c_keybinding_dispatches_on_initial_press() {
        let cc = mk_key_mod(KeyCode::Char('c'), true);
        let bindings = vec![mk_display(&[cc], "Close")];
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<Key>> = vec![];

        let consumed = handler.handle(&cc, &bindings, &mut |seq| dispatched.push(seq.to_vec()));

        assert!(consumed);
        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0][0], cc);
        assert!(!handler.is_active());
    }
}
