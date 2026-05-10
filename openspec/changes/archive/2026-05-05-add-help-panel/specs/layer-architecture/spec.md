# layer-architecture Specification (Delta)

## MODIFIED Requirements

### Requirement: Adding a new layer requires exactly three insertion points

To add a new layer (e.g., Help), the developer SHALL: (1) add a struct field to `App`, (2) add one `if` check in `handle_key_event`, (3) add one conditional render in `App::render`, (4) add one sync call in `App::sync`. No changes to existing layer logic SHALL be required.

#### Scenario: Adding a Help layer

- **WHEN** developer adds `help: HelpPanel` field to App (a struct wrapping a `visible: bool` with `is_active()`/`dismiss()`/`toggle()` methods)
- **AND** inserts `if self.help.is_active() { self.help.dismiss(); return; }` after PopUp check and before Chord check in handle_key_event
- **AND** inserts `if self.help.is_active() { Self::render_help(f, &tabs[ti]); }` in render
- **AND** Help requires no sync (state is purely synchronous)
- **THEN** Help functions as an independent layer without modifying PopUp, Which, or Global code

### Requirement: Layer priority is fully defined by insertion position

A layer's priority SHALL be determined by its position in the `handle_key_event` check sequence. Earlier checks = higher priority. PopUp is highest, Global is last (always called as fallback after Tab).

#### Scenario: Help layer between PopUp and Chord

- **WHEN** Help check is inserted after PopUp and before Chord
- **THEN** Help has higher priority than Chord, Tab, and Global (Help blocks all lower keys while open)
- **AND** Help has lower priority than PopUp (PopUp dialogs still work while Help is open)

### Requirement: Each layer independently decides key consumption

Each layer SHALL internally decide whether to consume a key event. No cross-layer coordination (e.g., "consumed" state flags passed between layers) is needed. If a layer returns/acts on a key, it SHALL `return` to stop further routing.

#### Scenario: Independent layer decisions

- **WHEN** PopUp handles a key and returns
- **THEN** Which, Tab, and Global are not called for that key event
- **WHEN** Which cancels chord on a non-matching key
- **THEN** the key is consumed, Tab and Global are not called

#### Scenario: Help dismiss consumes key

- **WHEN** Help is visible and user presses any key
- **THEN** Help calls `self.help.dismiss()` and returns
- **AND** Chord, Tab, and Global are not called for that key event
