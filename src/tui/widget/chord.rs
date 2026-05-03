use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

use super::tab::KeyCombo;

#[derive(Default)]
pub struct ChordHandler {
    pub pressed: Vec<KeyEvent>,
    pub candidates: Vec<(KeyCombo, &'static str)>,
}

impl ChordHandler {
    pub fn is_active(&self) -> bool {
        !self.pressed.is_empty()
    }

    pub fn handle(
        &mut self,
        kv: &KeyEvent,
        shortcuts: &[(KeyCombo, &'static str)],
        dispatch: &mut dyn FnMut(&[KeyEvent]),
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

    fn continue_(&mut self, kv: &KeyEvent, dispatch: &mut dyn FnMut(&[KeyEvent])) -> bool {
        if kv.code == KeyCode::Esc {
            self.reset();
            return true;
        }

        let idx = self.pressed.len();
        self.pressed.push(kv.clone());
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
        kv: &KeyEvent,
        shortcuts: &[(KeyCombo, &'static str)],
        dispatch: &mut dyn FnMut(&[KeyEvent]),
    ) -> bool {
        if kv.kind != KeyEventKind::Press {
            return false;
        }

        for (seq, _) in shortcuts {
            if seq.len() == 1 && seq[0] == *kv {
                dispatch(&[kv.clone()]);
                return true;
            }
        }

        let candidates: Vec<(KeyCombo, &str)> = shortcuts
            .iter()
            .filter(|(seq, _)| seq.len() > 1 && seq[0] == *kv)
            .cloned()
            .collect();

        if !candidates.is_empty() {
            self.pressed = vec![kv.clone()];
            self.candidates = candidates;
            return true;
        }

        false
    }
}

pub fn key_event_to_str(k: &KeyEvent) -> String {
    match k.code {
        KeyCode::Char(' ') => "<Space>".into(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "<Enter>".into(),
        KeyCode::Esc => "<Esc>".into(),
        KeyCode::Tab => "<Tab>".into(),
        KeyCode::Backspace => "<BS>".into(),
        KeyCode::Delete => "<Del>".into(),
        KeyCode::Right => "<Right>".into(),
        KeyCode::Left => "<Left>".into(),
        KeyCode::Up => "<Up>".into(),
        KeyCode::Down => "<Down>".into(),
        _ => format!("{:?}", k.code),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kev(code: KeyCode) -> KeyEvent {
        KeyEvent::new_with_kind_and_state(
            code,
            crossterm::event::KeyModifiers::empty(),
            KeyEventKind::Press,
            crossterm::event::KeyEventState::empty(),
        )
    }

    fn make_shortcuts(data: &[(&[KeyCode], &'static str)]) -> Vec<(KeyCombo, &'static str)> {
        data.iter()
            .map(|(codes, desc)| {
                (
                    KeyCombo(codes.iter().copied().map(kev).collect()),
                    *desc,
                )
            })
            .collect()
    }

    #[test]
    fn chord_init_single_key_dispatches() {
        let g = kev(KeyCode::Char('g'));
        let shortcuts = make_shortcuts(&[(&[KeyCode::Char('g')], "Action")]);
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<KeyEvent>> = vec![];

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
        let g = kev(KeyCode::Char('g'));
        let shortcuts = make_shortcuts(&[(&[KeyCode::Char('g'), KeyCode::Char('g')], "GoTop")]);
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<KeyEvent>> = vec![];

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
        let g = kev(KeyCode::Char('g'));
        let shortcuts = make_shortcuts(&[(&[KeyCode::Char('g'), KeyCode::Char('g')], "GoTop")]);
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<KeyEvent>> = vec![];

        handler.handle(&g, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        let consumed = handler.handle(&g, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));

        assert!(consumed);
        assert_eq!(dispatched.len(), 1);
        assert!(!handler.is_active());
    }

    #[test]
    fn chord_continue_non_matching_cancels_and_consumes() {
        let g = kev(KeyCode::Char('g'));
        let x = kev(KeyCode::Char('x'));
        let shortcuts = make_shortcuts(&[(&[KeyCode::Char('g'), KeyCode::Char('g')], "GoTop")]);
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<KeyEvent>> = vec![];

        handler.handle(&g, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        let consumed = handler.handle(&x, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));

        assert!(consumed);
        assert!(dispatched.is_empty());
        assert!(!handler.is_active());
    }

    #[test]
    fn chord_esc_cancels_and_consumes() {
        let g = kev(KeyCode::Char('g'));
        let esc = kev(KeyCode::Esc);
        let shortcuts = make_shortcuts(&[(&[KeyCode::Char('g'), KeyCode::Char('g')], "GoTop")]);
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<KeyEvent>> = vec![];

        handler.handle(&g, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        let consumed =
            handler.handle(&esc, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));

        assert!(consumed);
        assert!(dispatched.is_empty());
        assert!(!handler.is_active());
    }

    #[test]
    fn single_key_shortcut_takes_priority_over_chord_prefix() {
        let d = kev(KeyCode::Char('d'));
        let shortcuts = make_shortcuts(&[
            (&[KeyCode::Char('d')], "Delete"),
            (&[KeyCode::Char('d'), KeyCode::Char('d')], "DeleteAll"),
        ]);
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<KeyEvent>> = vec![];

        let consumed =
            handler.handle(&d, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));

        assert!(consumed);
        assert_eq!(dispatched.len(), 1);
        assert!(!handler.is_active());
    }

    #[test]
    fn exact_match_dispatches_among_multiple_candidates() {
        let g = kev(KeyCode::Char('g'));
        let e = kev(KeyCode::Char('e'));
        let shortcuts = make_shortcuts(&[
            (&[KeyCode::Char('g'), KeyCode::Char('g')], "GoTop"),
            (&[KeyCode::Char('g'), KeyCode::Char('e')], "GoEnd"),
        ]);
        let mut handler = ChordHandler::default();
        let mut dispatched: Vec<Vec<KeyEvent>> = vec![];

        handler.handle(&g, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));
        let consumed = handler.handle(&e, &shortcuts, &mut |seq| dispatched.push(seq.to_vec()));

        assert!(consumed);
        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0].len(), 2);
        assert!(!handler.is_active());
    }
}
