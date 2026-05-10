# chord-keybinding Specification

## Purpose
TBD - created by archiving change chord-keybinding-system. Update Purpose after archive.
## Requirements
### Requirement: Key type represents normalized keyboard input
The system SHALL define a `Key` struct with fields `code: KeyCode`, `shift: bool`, `ctrl: bool`, `alt: bool`, `super_: bool`. The type SHALL be constructable from `crossterm::event::KeyEvent` via `From<KeyEvent>`, normalizing platform-specific modifier behavior so that the same physical key press produces the same `Key` on all platforms. The type SHALL implement `Display` to render key names (e.g. `<C-Space>`, `<A-x>`, `g`).

#### Scenario: Plain character key
- **WHEN** user presses the `a` key with no modifiers
- **THEN** the resulting Key has `code = KeyCode::Char('a')` and all modifier flags are `false`

#### Scenario: Ctrl-modified key
- **WHEN** user presses `Ctrl-w`
- **THEN** the resulting Key has `code = KeyCode::Char('w')` and `ctrl = true`

#### Scenario: Shift-normalized across platforms
- **WHEN** user presses `Shift-a` (resulting in `KeyEvent` with `Char('A')` and `SHIFT` modifier on Windows)
- **THEN** the resulting Key has `code = KeyCode::Char('a')` and `shift = true`, matching the Unix representation

### Requirement: Chord defines a key sequence bound to commands
The system SHALL define a `Chord` struct with `on: Vec<Key>` (the key sequence to match) and `run: Vec<Cmd>` (the commands to execute when matched). A Chord with `on.len() == 1` SHALL execute immediately on a matching single key press. A Chord with `on.len() > 1` SHALL activate the Which panel on the first matching key and require subsequent keys to complete the sequence.

#### Scenario: Single-key chord executes immediately
- **WHEN** the active layer contains a Chord with `on = [Key::from('j')]` and user presses `j`
- **THEN** the Router matches the Chord and executes its commands immediately

#### Scenario: Multi-key chord activates Which
- **WHEN** the active layer contains a Chord with `on = [g, g]` and user presses `g`
- **THEN** the Router recognizes the prefix match, activates the Which panel, and does NOT execute any command yet

#### Scenario: Multi-key chord completes and executes
- **WHEN** Which is active with a Chord `on = [g, g]` and user presses `g` again
- **THEN** the Router matches the full sequence and executes the Chord's commands

### Requirement: Layer-based chord routing
The system SHALL support multiple `Layer` values (at minimum: `App`, plus one per tab). Each Layer SHALL have its own `Vec<Chord>` registry. The Router SHALL match incoming keys against only the currently active layer's chords. The `App` layer SHALL be checked first for global keys (tab switching, quit), then the current tab's layer.

#### Scenario: Global key takes priority
- **WHEN** the `App` layer has Chord `on = [q]` (quit) and the Status layer also has Chord `on = [q]` (some tab action), and user presses `q`
- **THEN** the App layer matches first and the quit command executes, not the tab action

#### Scenario: Tab-specific chord is layer-scoped
- **WHEN** the Status layer has Chord `on = [r]` for refresh and the File layer does not, and user presses `r` while on FileTab
- **THEN** nothing happens (no match in File layer)

### Requirement: TuiWidget contract changes to chord registration + command execution
The `TuiWidget` trait SHALL no longer expose `handle_key_event(&mut self, kv: &KeyEvent)`. Instead, each TuiWidget SHALL provide a method to enumerate its Chords and a method to execute commands dispatched by the Router. Tabs SHALL NOT receive raw key events.

#### Scenario: Tab registers chords
- **WHEN** a `StatusTab` implements `TuiWidget`
- **THEN** it provides a `chords() -> &[Chord]` method returning all its keybindings

#### Scenario: Router dispatches command to tab
- **WHEN** the Router matches a Chord in the Status layer
- **THEN** it calls `execute(cmd)` on the active tab with the Chord's `run` commands

### Requirement: Router is the single key event entry point
The `App` SHALL route all `KeyEvent`s through the `Router` before any other handler. The routing order SHALL be: Which (if active) → PopUp → Router (App layer → Tab layer). The Router SHALL return a boolean indicating whether the key was consumed.

#### Scenario: Key consumed by Router
- **WHEN** the Router matches a Chord and executes its commands
- **THEN** the Router returns `true` and no further handlers process the key

#### Scenario: Key not consumed by Router
- **WHEN** no Chord matches the pressed key in any layer
- **THEN** the Router returns `false` and the key may be handled by fallback logic

