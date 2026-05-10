# shortcut-bindings Specification (Delta)

## ADDED Requirements

### Requirement: mod_agent! accepts modifier key syntax

The `mod_agent!` macro SHALL accept angle-bracket key syntax with modifier prefixes for single-key and multi-key entries. Supported prefixes: `C-` (Ctrl), `A-` (Alt), `S-` (Shift), `D-` (Super/Meta). A bare character entry (e.g., `KeyCode::Char('j')` without modifiers) SHALL produce a Key with `KeyModifiers::empty()`. Entries using the existing `KeyCode` format SHALL continue to work unchanged, producing Keys with all modifiers `false`.

#### Scenario: Ctrl-modified key in mod_agent!
- **WHEN** `mod_agent!` is invoked with `(<C-q>, Key::Quit, "Quit")`
- **THEN** the generated Agent maps `Key { code: Char('q'), ctrl: true, ... }` to `Key::Quit`

#### Scenario: Multi-modifier key in mod_agent!
- **WHEN** `mod_agent!` is invoked with `(<C-S-p>, Key::Action(Apply), "Apply")`
- **THEN** the generated Agent maps `Key { code: Char('p'), ctrl: true, shift: true, ... }` to `Key::Action(Apply)`

#### Scenario: Bare KeyCode still works unchanged
- **WHEN** `mod_agent!` is invoked with `(KeyCode::Char('j'), Key::MoveDown, "Move down")`
- **THEN** the generated Agent maps `Key { code: Char('j'), ctrl: false, alt: false, shift: false, super_: false }` to `Key::MoveDown`

#### Scenario: Bare character with uppercase for Shift
- **WHEN** `mod_agent!` is invoked with `(KeyCode::Char('J'), Key::MoveDown, "Move down")`
- **THEN** the shift flag is derived from `is_ascii_uppercase()`, producing `Key { code: Char('J'), shift: true }`

#### Scenario: Special key with modifier in mod_agent!
- **WHEN** `mod_agent!` is invoked with `(<S-Up>, Key::MoveUp, "Move up")`
- **THEN** the generated Agent maps `Key { code: Up, shift: true }` to `Key::MoveUp`

### Requirement: KeyCombo wraps Vec<Key> instead of Vec<KeyEvent>

The `KeyCombo` type SHALL be defined as `KeyCombo(Vec<Key>)` where `Key` is the new normalized key type. `KeyCombo` SHALL provide `Deref<Target = [Key]>` for slice access. `KeyCombo` SHALL derive `PartialEq, Eq, Hash` via the derive on `Key`.

#### Scenario: KeyCombo equality
- **WHEN** comparing `KeyCombo(vec![Key::from_str("<C-c>")])` with `KeyCombo(vec![Key::from_str("<C-c>")])`
- **THEN** they are equal

#### Scenario: KeyCombo different from bare key
- **WHEN** comparing `KeyCombo(vec![Key::from_str("<C-c>")])` with `KeyCombo(vec![Key::from_str("c")])`
- **THEN** they are NOT equal (Ctrl-c vs plain c)

## MODIFIED Requirements

### Requirement: mod_agent! generates unified shortcuts

The `mod_agent!` macro SHALL generate a `shortcuts()` function that returns ALL key bindings (single-key and multi-key) as a slice of `(key_sequence, action, description)` tuples where `key_sequence` is `Vec<Key>`. The existing `agent()` and `agent_init()` SHALL remain for keymap.yaml compatibility but SHALL use `Key` as the HashMap key type instead of `KeyEvent`.

#### Scenario: Single-key binding in shortcuts
- **WHEN** `mod_agent!` is invoked with `(KeyCode::Char('i'), Key::Action(Action::Add), "Add profile")`
- **THEN** `shortcuts()` returns an entry `(vec![Key { code: Char('i'), ... }], Key::Action(Add), "Add profile")`

#### Scenario: Multi-key chord in shortcuts
- **WHEN** `mod_agent!` is invoked with `(KeyCode::Char('g'), KeyCode::Char('g'), Key::Action(Action::GoTop), "Go to top")`
- **THEN** `shortcuts()` returns an entry `(vec![Key { code: Char('g'), ... }, Key { code: Char('g'), ... }], Key::Action(GoTop), "Go to top")`

#### Scenario: Modifier key in shortcuts
- **WHEN** `mod_agent!` is invoked with `(<C-c>, Key::Close, "Close tab")`
- **THEN** `shortcuts()` returns an entry `(vec![Key { code: Char('c'), ctrl: true }], Key::Close, "Close tab")`

#### Scenario: Mixed single-key and chord entries
- **WHEN** a tab defines both single-key bindings and chord bindings in `mod_agent!`
- **THEN** `shortcuts()` returns one consolidated slice containing both types as `Vec<(Vec<Key>, &str)>`

### Requirement: Focused panel exposes shortcuts and dispatch

Each tab SHALL implement `shortcuts() -> &[(Vec<Key>, &str)]` returning the focused panel's shortcut table (key sequences use `Key` instead of `KeyEvent`). Each tab SHALL implement `dispatch_shortcut(&mut self, seq: &[Key]) -> bool` to match a key sequence against the focused panel's shortcuts and execute the corresponding action.

#### Scenario: Tab<C> shortcuts from content
- **WHEN** `Tab<C>::shortcuts()` is called
- **THEN** it returns `C::shortcuts()` mapped to `(seq, desc)` pairs where `seq` is `Vec<Key>`

#### Scenario: Tab<C> dispatch_shortcut matches a sequence
- **WHEN** `Tab<C>::dispatch_shortcut(&[Key { code: Enter, .. }])` is called and `Enter` maps to `Key::Action(Apply)`
- **THEN** the tab resolves the sequence to `Key::Action(Apply)`, calls `content.handle_key_event(Key::Action(Apply), ...)`, and returns `true`

#### Scenario: DualTab shortcuts from focused pane
- **WHEN** `DualTab<C1,C2>::shortcuts()` is called and `is_focus_on_c1` is true
- **THEN** it returns `C1::shortcuts()` mapped to `(seq, desc)` pairs where `seq` is `Vec<Key>`

#### Scenario: DualTab shortcuts from mate pane
- **WHEN** `DualTab<C1,C2>::shortcuts()` is called and `is_focus_on_c1` is false
- **THEN** it returns `C2::shortcuts()` mapped to `(seq, desc)` pairs where `seq` is `Vec<Key>`

#### Scenario: dispatch_shortcut returns false for unknown sequence
- **WHEN** `dispatch_shortcut` is called with a key sequence not in the shortcuts table
- **THEN** it returns `false` and does not modify any state

### Requirement: Existing single-key agent unchanged

The existing `agent()` static HashMap and `agent_init()` function SHALL use `Key` as the key type instead of `crossterm::KeyEvent`. The `TryFrom<&Key> for CustomKeyEnum` implementations SHALL use `agent()` for key lookup (changing from `TryFrom<&KeyEvent>` to `TryFrom<&Key>`).

#### Scenario: keymap.yaml still loads single-key bindings
- **WHEN** `agent_init(map)` is called with a serialized HashMap from `keymap.yaml` using `Key` as keys
- **THEN** the agent static is populated and `TryFrom<&Key>` uses it for key resolution
