## ADDED Requirements

### Requirement: Layers have a unified interface pattern
Each overlay layer in App SHALL follow the same structural pattern: a struct field on App, checked in handle_key_event/render/sync in fixed priority order. Each layer SHALL provide `is_active()` to determine if it should intercept keys and render.

#### Scenario: PopUp layer follows the pattern
- **WHEN** implementing PopUp as a layer
- **THEN** App has a `popup: PopUp` field, checked via `if self.popup.check()` in handle_key_event, rendered via `self.popup.render(f, area)`, synced via `self.popup.sync()`

#### Scenario: Which layer follows the pattern
- **WHEN** implementing Which as a layer
- **THEN** App has a `chord: ChordHandler` field, checked via `if self.chord.handle(kv, shortcuts, dispatch)` in handle_key_event, rendered when `chord.is_active()`, no sync needed (chord is sync)

### Requirement: Adding a new layer requires exactly three insertion points
To add a new layer (e.g., Help), the developer SHALL: (1) add a struct field to `App`, (2) add one `if` check in `handle_key_event`, (3) add one conditional render in `App::render`, (4) add one sync call in `App::sync`. No changes to existing layer logic SHALL be required.

#### Scenario: Adding a Help layer
- **WHEN** developer adds `help: HelpPanel` field to App
- **AND** inserts `if self.help.is_active() { self.help.handle_key_event(kv); return; }` after global keys
- **AND** inserts `if self.help.is_active() { self.help.render(f, area); }` in render
- **AND** inserts `self.help.sync();` in sync
- **THEN** Help functions as an independent layer without modifying PopUp, Which, or Global code

### Requirement: Layer priority is fully defined by insertion position
A layer's priority SHALL be determined by its position in the `handle_key_event` check sequence. Earlier checks = higher priority. PopUp is highest, Global is last (always called as fallback after Tab).

#### Scenario: Help layer between Tab and Global
- **WHEN** Help check is inserted after Tab and before Global
- **THEN** Help has higher priority than Global (Help blocks global keys while open)
- **AND** Help has lower priority than Tab (tab keys work while Help is open)

### Requirement: Each layer independently decides key consumption
Each layer SHALL internally decide whether to consume a key event. No cross-layer coordination (e.g., "consumed" state flags passed between layers) is needed. If a layer returns/acts on a key, it SHALL `return` to stop further routing.

#### Scenario: Independent layer decisions
- **WHEN** PopUp handles a key and returns
- **THEN** Which, Tab, and Global are not called for that key event
- **WHEN** Which cancels chord on a non-matching key
- **THEN** the key is consumed, Tab and Global are not called
