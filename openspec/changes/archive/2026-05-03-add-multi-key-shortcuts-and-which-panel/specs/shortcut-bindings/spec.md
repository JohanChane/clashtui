## ADDED Requirements

### Requirement: mod_agent! generates unified shortcuts

The `mod_agent!` macro SHALL generate a `shortcuts()` function that returns ALL key bindings (single-key and multi-key) as a slice of `(key_sequence, action, description)` tuples. The existing `agent()` and `agent_init()` SHALL remain for keymap.yaml compatibility.

#### Scenario: Single-key binding in shortcuts
- **WHEN** `mod_agent!` is invoked with `(KeyCode::Char('i'), Key::Action(Action::Add), "Add profile")`
- **THEN** `shortcuts()` returns an entry `(vec![KeyEvent('i', Press)], Key::Action(Add), "Add profile")`

#### Scenario: Multi-key chord in shortcuts
- **WHEN** `mod_agent!` is invoked with `(KeyCode::Char('g'), KeyCode::Char('g'), Key::Action(Action::GoTop), "Go to top")`
- **THEN** `shortcuts()` returns an entry `(vec![KeyEvent('g', Press), KeyEvent('g', Press)], Key::Action(GoTop), "Go to top")`

#### Scenario: Mixed single-key and chord entries
- **WHEN** a tab defines both single-key bindings and chord bindings in `mod_agent!`
- **THEN** `shortcuts()` returns one consolidated slice containing both types

### Requirement: Focused panel exposes shortcuts and dispatch

Each tab SHALL implement `shortcuts() -> Vec<(Vec<KeyEvent>, &str)>` returning the focused panel's shortcut table (without the type-specific Key). Each tab SHALL implement `dispatch_shortcut(&mut self, seq: &[KeyEvent]) -> bool` to match a key sequence against the focused panel's shortcuts and execute the corresponding action.

#### Scenario: Tab<C> shortcuts from content
- **WHEN** `Tab<C>::shortcuts()` is called
- **THEN** it returns `C::shortcuts()` mapped to `(seq, desc)` pairs

#### Scenario: Tab<C> dispatch_shortcut matches a sequence
- **WHEN** `Tab<C>::dispatch_shortcut(&[KeyEvent(Enter, Press)])` is called and `Enter` maps to `Key::Action(Apply)`
- **THEN** the tab resolves the sequence to `Key::Action(Apply)`, calls `content.handle_key_event(Key::Action(Apply), ...)`, and returns `true`

#### Scenario: DualTab shortcuts from focused pane
- **WHEN** `DualTab<C1,C2>::shortcuts()` is called and `is_focus_on_c1` is true
- **THEN** it returns `C1::shortcuts()` mapped to `(seq, desc)` pairs

#### Scenario: DualTab shortcuts from mate pane
- **WHEN** `DualTab<C1,C2>::shortcuts()` is called and `is_focus_on_c1` is false
- **THEN** it returns `C2::shortcuts()` mapped to `(seq, desc)` pairs

#### Scenario: dispatch_shortcut returns false for unknown sequence
- **WHEN** `dispatch_shortcut` is called with a key sequence not in the shortcuts table
- **THEN** it returns `false` and does not modify any state

### Requirement: Existing single-key agent unchanged

The existing `agent()` static HashMap and `agent_init()` function SHALL continue to work for loading key bindings from `keymap.yaml`. The `TryFrom<&KeyEvent> for Key` implementations SHALL remain unchanged and continue to use `agent()` for key lookup.

#### Scenario: keymap.yaml still loads single-key bindings
- **WHEN** `agent_init(map)` is called with a serialized HashMap from `keymap.yaml`
- **THEN** the agent static is populated and `TryFrom<&KeyEvent>` uses it for key resolution
