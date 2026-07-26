use crate::tui::Key;
use std::collections::HashSet;
use std::fmt;

// -- Binding --

/// A key binding: key sequence -> action + optional description.
/// `on.len() == 1` is a single key, `on.len() >= 2` is a chord.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Binding<A> {
    pub on: Vec<Key>,
    pub exec: A,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
}

// -- DisplayBinding --

/// Action-stripped display type. ChordHandler, help panel, and Which popup
/// only store this -- keeping ChordHandler non-generic.
#[derive(Debug, Clone)]
pub struct DisplayBinding {
    pub on: Vec<Key>,
    pub desc: String,
}

impl<A> From<&Binding<A>> for DisplayBinding {
    fn from(b: &Binding<A>) -> Self {
        Self {
            on: b.on.clone(),
            desc: b.desc.clone().unwrap_or_default(),
        }
    }
}

// -- Scope --

/// The context in which a key binding is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    Global,
    Connections,
    Proxies,
    SrvCtl,
    Settings,
    Logs,
    FileProfile,
    FileTemplate,
    Status,
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Global => write!(f, "global"),
            Self::Connections => write!(f, "connections"),
            Self::Proxies => write!(f, "proxies"),
            Self::SrvCtl => write!(f, "srvctl"),
            Self::Settings => write!(f, "settings"),
            Self::Logs => write!(f, "logs"),
            Self::FileProfile => write!(f, "file/profile"),
            Self::FileTemplate => write!(f, "file/template"),
            Self::Status => write!(f, "status"),
        }
    }
}

// -- Keymap --

/// A single scope's key binding collection.
/// User overrides appear first, followed by retained defaults.
#[derive(Debug, Clone)]
pub struct Keymap<A> {
    bindings: Vec<Binding<A>>,
}

impl<A: Clone> Keymap<A> {
    /// Merge defaults + user overrides, then validate no conflicts.
    /// User overrides take priority -- defaults with the same `on` sequence
    /// are replaced. After merge, we run exact-duplicate + prefix-conflict
    /// + max-depth validations.
    pub fn merge(
        defaults: &[Binding<A>],
        overrides: &[Binding<A>],
        scope: Scope,
    ) -> anyhow::Result<Self> {
        let override_keys: HashSet<&Vec<Key>> = overrides.iter().map(|b| &b.on).collect();
        let mut bindings: Vec<Binding<A>> = overrides.to_vec();
        bindings.extend(
            defaults
                .iter()
                .filter(|b| !override_keys.contains(&b.on))
                .cloned(),
        );
        validate_no_exact_duplicates(&bindings, scope)?;
        validate_no_prefix_conflicts(&bindings, scope)?;
        validate_max_depth(&bindings, scope, 2)?;
        Ok(Self { bindings })
    }

    #[allow(dead_code)]
    pub fn bindings(&self) -> &[Binding<A>] {
        &self.bindings
    }

    /// Look up a key sequence (O(n), n <= 40).
    pub fn find_by_seq(&self, seq: &[Key]) -> Option<&A> {
        self.bindings.iter().find(|b| b.on == seq).map(|b| &b.exec)
    }

    /// Generate DisplayBinding list, cached for ChordHandler / help panel / Which popup.
    pub fn to_display(&self) -> Vec<DisplayBinding> {
        self.bindings.iter().map(DisplayBinding::from).collect()
    }
}

// -- Validation --

/// Exact duplicate check: within one scope, no two bindings may have the
/// identical `on` sequence.
fn validate_no_exact_duplicates<A>(bindings: &[Binding<A>], scope: Scope) -> anyhow::Result<()> {
    use std::collections::HashMap;
    let mut seen: HashMap<&Vec<Key>, (&Binding<A>, usize)> = HashMap::new();
    for (i, b) in bindings.iter().enumerate() {
        if let Some((first, first_idx)) = seen.get(&b.on) {
            let seq = crate::tui::format_key_sequence(&b.on);
            let a_desc = first.desc.as_deref().unwrap_or("<no description>");
            let b_desc = b.desc.as_deref().unwrap_or("<no description>");
            anyhow::bail!(
                "[{scope}] duplicate key binding: {seq}\n  \
                 First binding  (entry #{first_idx}): \"{a_desc}\"\n  \
                 Second binding (entry #{i}): \"{b_desc}\"\n  \
                 Each key sequence must be unique within a scope. \
                 Remove or change one of the conflicting entries.",
                first_idx = first_idx + 1,
                i = i + 1,
            );
        }
        seen.insert(&b.on, (b, i));
    }
    Ok(())
}

/// Prefix-conflict check: within one scope, no binding's `on` may be a
/// prefix of another binding's `on`. This guarantees ChordHandler never
/// needs to decide "single-key vs longer chord" at runtime.
fn validate_no_prefix_conflicts<A>(bindings: &[Binding<A>], scope: Scope) -> anyhow::Result<()> {
    for (i, a) in bindings.iter().enumerate() {
        for (j, b) in bindings.iter().enumerate().skip(i + 1) {
            if a.on.starts_with(&b.on) || b.on.starts_with(&a.on) {
                let a_seq = crate::tui::format_key_sequence(&a.on);
                let b_seq = crate::tui::format_key_sequence(&b.on);
                let a_desc = a.desc.as_deref().unwrap_or("<no description>");
                let b_desc = b.desc.as_deref().unwrap_or("<no description>");
                anyhow::bail!(
                    "[{scope}] prefix conflict: key sequences share a common prefix\n  \
                     Entry #{i}: {a_seq} -> \"{a_desc}\"\n  \
                     Entry #{j}: {b_seq} -> \"{b_desc}\"\n  \
                     A single-key binding cannot be a prefix of a chord (e.g. \"d\" \
                     conflicts with [\"d\",\"d\"]). Remove one or use \
                     non-overlapping key sequences.",
                    i = i + 1,
                    j = j + 1,
                );
            }
        }
    }
    Ok(())
}

/// Chord depth check: reject any `on` sequence longer than `max`.
/// Modifiers don't count toward depth (`<C-g>` is one Key), so 2 keys
/// suffice for a dashboard TUI.
fn validate_max_depth<A>(bindings: &[Binding<A>], scope: Scope, max: usize) -> anyhow::Result<()> {
    for b in bindings {
        if b.on.len() > max {
            let seq = crate::tui::format_key_sequence(&b.on);
            let desc = b.desc.as_deref().unwrap_or("<no description>");
            anyhow::bail!(
                "[{scope}] chord too deep: {seq} -> \"{desc}\" has {} keys \
                 (max {max}). Modifiers don't count toward depth -- \
                 <C-g> is one key.",
                b.on.len(),
            );
        }
    }
    Ok(())
}

// -- Tests --

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyCode;

    fn mk_key(code: KeyCode) -> Key {
        Key {
            code,
            shift: matches!(code, KeyCode::Char(c) if c.is_ascii_uppercase()),
            ctrl: false,
            alt: false,
            super_: false,
        }
    }

    fn ctrl(c: char) -> Key {
        Key {
            code: KeyCode::Char(c),
            shift: false,
            ctrl: true,
            alt: false,
            super_: false,
        }
    }

    fn binding<A: Clone>(on: Vec<Key>, exec: A, desc: &str) -> Binding<A> {
        Binding {
            on,
            exec,
            desc: if desc.is_empty() {
                None
            } else {
                Some(desc.into())
            },
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    enum TestAction {
        MoveUp,
        MoveDown,
        GoTop,
        Quit,
    }

    // -- Deserialization tests for Binding/RawEntry are in agent.rs --
    // (they need the deserialize_on visitor which is private to agent.rs)

    // -- DisplayBinding --

    #[test]
    fn display_binding_from_binding() {
        let b = binding(
            vec![mk_key(KeyCode::Char('j'))],
            TestAction::MoveDown,
            "Move down",
        );
        let d = DisplayBinding::from(&b);
        assert_eq!(d.on, b.on);
        assert_eq!(d.desc, "Move down");
    }

    #[test]
    fn display_binding_empty_desc() {
        let b = binding(vec![mk_key(KeyCode::Char('q'))], TestAction::Quit, "");
        let d = DisplayBinding::from(&b);
        assert_eq!(d.desc, "");
    }

    // -- Scope Display --

    #[test]
    fn scope_display_strings() {
        assert_eq!(Scope::Global.to_string(), "global");
        assert_eq!(Scope::Connections.to_string(), "connections");
        assert_eq!(Scope::Proxies.to_string(), "proxies");
        assert_eq!(Scope::SrvCtl.to_string(), "srvctl");
        assert_eq!(Scope::Settings.to_string(), "settings");
        assert_eq!(Scope::Logs.to_string(), "logs");
        assert_eq!(Scope::FileProfile.to_string(), "file/profile");
        assert_eq!(Scope::FileTemplate.to_string(), "file/template");
        assert_eq!(Scope::Status.to_string(), "status");
    }

    // -- Keymap::merge --

    #[test]
    fn merge_user_override_replaces_default() {
        let defaults = vec![
            binding(
                vec![mk_key(KeyCode::Char('j'))],
                TestAction::MoveDown,
                "Move down",
            ),
            binding(
                vec![mk_key(KeyCode::Char('k'))],
                TestAction::MoveUp,
                "Move up",
            ),
        ];
        let overrides = vec![binding(
            vec![mk_key(KeyCode::Char('j'))],
            TestAction::GoTop,
            "Go top",
        )];
        let km = Keymap::merge(&defaults, &overrides, Scope::Connections).unwrap();
        let bindings = km.bindings();
        // overrides come first
        assert_eq!(bindings[0].exec, TestAction::GoTop);
        // then retained defaults
        assert_eq!(bindings[1].exec, TestAction::MoveUp);
        assert_eq!(bindings.len(), 2);
    }

    #[test]
    fn merge_user_adds_new_binding() {
        let defaults = vec![binding(
            vec![mk_key(KeyCode::Char('j'))],
            TestAction::MoveDown,
            "Move down",
        )];
        let overrides = vec![binding(
            vec![mk_key(KeyCode::Char('q'))],
            TestAction::Quit,
            "Quit",
        )];
        let km = Keymap::merge(&defaults, &overrides, Scope::Connections).unwrap();
        let bindings = km.bindings();
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0].exec, TestAction::Quit);
        assert_eq!(bindings[1].exec, TestAction::MoveDown);
    }

    #[test]
    fn merge_defaults_only() {
        let defaults = vec![binding(
            vec![mk_key(KeyCode::Char('j'))],
            TestAction::MoveDown,
            "Move down",
        )];
        let km = Keymap::merge(&defaults, &[], Scope::Connections).unwrap();
        assert_eq!(km.bindings().len(), 1);
    }

    // -- Keymap::find_by_seq --

    #[test]
    fn find_by_seq_single_key() {
        let defaults = vec![binding(
            vec![mk_key(KeyCode::Char('j'))],
            TestAction::MoveDown,
            "Move down",
        )];
        let km = Keymap::merge(&defaults, &[], Scope::Connections).unwrap();
        let found = km.find_by_seq(&[mk_key(KeyCode::Char('j'))]);
        assert_eq!(found, Some(&TestAction::MoveDown));
    }

    #[test]
    fn find_by_seq_chord() {
        let defaults = vec![binding(
            vec![mk_key(KeyCode::Char('g')), mk_key(KeyCode::Char('g'))],
            TestAction::GoTop,
            "Go top",
        )];
        let km = Keymap::merge(&defaults, &[], Scope::Connections).unwrap();
        let found = km.find_by_seq(&[mk_key(KeyCode::Char('g')), mk_key(KeyCode::Char('g'))]);
        assert_eq!(found, Some(&TestAction::GoTop));
    }

    #[test]
    fn find_by_seq_not_found() {
        let defaults = vec![binding(
            vec![mk_key(KeyCode::Char('j'))],
            TestAction::MoveDown,
            "Move down",
        )];
        let km = Keymap::merge(&defaults, &[], Scope::Connections).unwrap();
        assert!(km.find_by_seq(&[mk_key(KeyCode::Char('x'))]).is_none());
    }

    // -- to_display --

    #[test]
    fn to_display_returns_display_bindings() {
        let defaults = vec![
            binding(
                vec![mk_key(KeyCode::Char('j'))],
                TestAction::MoveDown,
                "Move down",
            ),
            binding(
                vec![mk_key(KeyCode::Char('g')), mk_key(KeyCode::Char('g'))],
                TestAction::GoTop,
                "Go top",
            ),
        ];
        let km = Keymap::merge(&defaults, &[], Scope::Connections).unwrap();
        let display = km.to_display();
        assert_eq!(display.len(), 2);
        assert_eq!(display[0].desc, "Move down");
        assert_eq!(display[1].desc, "Go top");
    }

    // -- validate_no_exact_duplicates --

    #[test]
    fn validate_no_exact_duplicates_ok() {
        let bindings = vec![
            binding(
                vec![mk_key(KeyCode::Char('j'))],
                TestAction::MoveDown,
                "Move down",
            ),
            binding(
                vec![mk_key(KeyCode::Char('k'))],
                TestAction::MoveUp,
                "Move up",
            ),
        ];
        assert!(validate_no_exact_duplicates(&bindings, Scope::Connections).is_ok());
    }

    #[test]
    fn validate_no_exact_duplicates_fails() {
        let bindings = vec![
            binding(
                vec![mk_key(KeyCode::Char('j'))],
                TestAction::MoveDown,
                "Move down",
            ),
            binding(
                vec![mk_key(KeyCode::Char('j'))],
                TestAction::MoveUp,
                "Move up",
            ),
        ];
        let err = validate_no_exact_duplicates(&bindings, Scope::Connections).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("[connections] duplicate key binding:"),
            "msg: {msg}"
        );
        assert!(msg.contains("j"), "msg should contain the key, got: {msg}");
        assert!(msg.contains("\"Move down\""), "msg: {msg}");
        assert!(msg.contains("\"Move up\""), "msg: {msg}");
        assert!(msg.contains("entry #1"), "msg: {msg}");
        assert!(msg.contains("entry #2"), "msg: {msg}");
    }

    #[test]
    fn validate_no_exact_duplicates_chord_shows_readable_keys() {
        // gg duplicated — the exact bug that triggered the error-message overhaul.
        let bindings = vec![
            binding(
                vec![mk_key(KeyCode::Char('g')), mk_key(KeyCode::Char('g'))],
                TestAction::GoTop,
                "Go to top",
            ),
            binding(
                vec![mk_key(KeyCode::Char('g')), mk_key(KeyCode::Char('g'))],
                TestAction::Quit,
                "Go to bottom",
            ),
        ];
        let err = validate_no_exact_duplicates(&bindings, Scope::Connections).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("g → g"), "expected readable chord, got: {msg}");
        assert!(
            !msg.contains("Key {"),
            "should not contain Debug format, got: {msg}"
        );
        assert!(msg.contains("\"Go to top\""), "msg: {msg}");
        assert!(msg.contains("\"Go to bottom\""), "msg: {msg}");
    }

    // -- validate_no_prefix_conflicts --

    #[test]
    fn validate_no_prefix_conflicts_ok() {
        let bindings = vec![
            binding(
                vec![mk_key(KeyCode::Char('j'))],
                TestAction::MoveDown,
                "Move down",
            ),
            binding(
                vec![mk_key(KeyCode::Char('k'))],
                TestAction::MoveUp,
                "Move up",
            ),
            binding(
                vec![mk_key(KeyCode::Char('g')), mk_key(KeyCode::Char('g'))],
                TestAction::GoTop,
                "Go top",
            ),
        ];
        assert!(validate_no_prefix_conflicts(&bindings, Scope::Connections).is_ok());
    }

    #[test]
    fn validate_no_prefix_conflicts_fails_single_versus_chord() {
        let bindings = vec![
            binding(
                vec![mk_key(KeyCode::Char('d'))],
                TestAction::MoveDown,
                "Delete",
            ),
            binding(
                vec![mk_key(KeyCode::Char('d')), mk_key(KeyCode::Char('d'))],
                TestAction::GoTop,
                "Delete all",
            ),
        ];
        let err = validate_no_prefix_conflicts(&bindings, Scope::Connections).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("[connections] prefix conflict"), "msg: {msg}");
        assert!(msg.contains("d → d"), "expected readable chord, got: {msg}");
        assert!(
            !msg.contains("Key {"),
            "should not contain Debug format, got: {msg}"
        );
        assert!(msg.contains("\"Delete\""), "msg: {msg}");
        assert!(msg.contains("\"Delete all\""), "msg: {msg}");
    }

    #[test]
    fn validate_no_prefix_conflicts_fails_multi_prefix() {
        let bindings = vec![
            binding(
                vec![mk_key(KeyCode::Char('g')), mk_key(KeyCode::Char('g'))],
                TestAction::GoTop,
                "Go top",
            ),
            binding(
                vec![
                    mk_key(KeyCode::Char('g')),
                    mk_key(KeyCode::Char('g')),
                    mk_key(KeyCode::Char('g')),
                ],
                TestAction::Quit,
                "Triple g",
            ),
        ];
        let err = validate_no_prefix_conflicts(&bindings, Scope::Connections).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("[connections] prefix conflict"), "msg: {msg}");
        assert!(msg.contains("\"Go top\""), "msg: {msg}");
        assert!(msg.contains("\"Triple g\""), "msg: {msg}");
    }

    #[test]
    fn validate_no_prefix_conflicts_ctrl_chord() {
        // <C-g>c and <C-g>f: first key (<C-g>) is same, but chords [<C-g>, c] and [<C-g>, f]
        // are same length -> no prefix conflict
        let bindings = vec![
            binding(
                vec![ctrl('g'), mk_key(KeyCode::Char('c'))],
                TestAction::MoveUp,
                "Open data dir",
            ),
            binding(
                vec![ctrl('g'), mk_key(KeyCode::Char('f'))],
                TestAction::MoveDown,
                "Restart",
            ),
        ];
        assert!(validate_no_prefix_conflicts(&bindings, Scope::Global).is_ok());
    }

    // -- validate_max_depth --

    #[test]
    fn validate_max_depth_passes_for_len_2() {
        let bindings = vec![binding(
            vec![mk_key(KeyCode::Char('g')), mk_key(KeyCode::Char('g'))],
            TestAction::GoTop,
            "Go top",
        )];
        assert!(validate_max_depth(&bindings, Scope::Connections, 2).is_ok());
    }

    #[test]
    fn validate_max_depth_passes_for_len_1() {
        let bindings = vec![binding(
            vec![mk_key(KeyCode::Char('j'))],
            TestAction::MoveDown,
            "Move down",
        )];
        assert!(validate_max_depth(&bindings, Scope::Connections, 2).is_ok());
    }

    #[test]
    fn validate_max_depth_fails_for_len_3() {
        let bindings = vec![binding(
            vec![
                mk_key(KeyCode::Char('g')),
                mk_key(KeyCode::Char('g')),
                mk_key(KeyCode::Char('g')),
            ],
            TestAction::GoTop,
            "Triple g",
        )];
        let err = validate_max_depth(&bindings, Scope::Connections, 2).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("[connections] chord too deep"), "msg: {msg}");
        assert!(
            msg.contains("g → g → g"),
            "expected readable chord, got: {msg}"
        );
        assert!(msg.contains("\"Triple g\""), "msg: {msg}");
        assert!(msg.contains("3 keys"), "msg: {msg}");
        assert!(msg.contains("max 2"), "msg: {msg}");
    }

    // -- merge validation integration --

    #[test]
    fn merge_rejects_duplicate_in_overrides() {
        let defaults: Vec<Binding<TestAction>> = vec![];
        let overrides = vec![
            binding(
                vec![mk_key(KeyCode::Char('j'))],
                TestAction::MoveDown,
                "first",
            ),
            binding(
                vec![mk_key(KeyCode::Char('j'))],
                TestAction::MoveUp,
                "second",
            ),
        ];
        assert!(Keymap::merge(&defaults, &overrides, Scope::Connections).is_err());
    }

    #[test]
    fn merge_rejects_prefix_conflict() {
        let defaults = vec![binding(
            vec![mk_key(KeyCode::Char('d'))],
            TestAction::MoveDown,
            "Delete",
        )];
        let overrides = vec![binding(
            vec![mk_key(KeyCode::Char('d')), mk_key(KeyCode::Char('d'))],
            TestAction::GoTop,
            "Delete all",
        )];
        assert!(Keymap::merge(&defaults, &overrides, Scope::Connections).is_err());
    }

    #[test]
    fn merge_rejects_deep_chord() {
        let defaults: Vec<Binding<TestAction>> = vec![];
        let overrides = vec![binding(
            vec![
                mk_key(KeyCode::Char('a')),
                mk_key(KeyCode::Char('b')),
                mk_key(KeyCode::Char('c')),
            ],
            TestAction::GoTop,
            "Triple chord",
        )];
        assert!(Keymap::merge(&defaults, &overrides, Scope::Connections).is_err());
    }

    // -- Cross-scope non-conflict --

    #[test]
    fn cross_scope_same_key_is_allowed() {
        // 'q' in Global = Quit, 'q' in Connections = TogglePause -- perfectly legal
        let global_defaults = vec![binding(
            vec![mk_key(KeyCode::Char('q'))],
            TestAction::Quit,
            "Quit",
        )];
        let conn_defaults = vec![binding(
            vec![mk_key(KeyCode::Char('q'))],
            TestAction::MoveDown,
            "Pause",
        )];
        assert!(Keymap::merge(&global_defaults, &[], Scope::Global).is_ok());
        assert!(Keymap::merge(&conn_defaults, &[], Scope::Connections).is_ok());
    }
}
