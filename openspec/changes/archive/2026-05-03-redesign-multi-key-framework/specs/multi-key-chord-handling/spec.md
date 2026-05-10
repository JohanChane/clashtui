## ADDED Requirements

### Requirement: Layer routing follows fixed priority order
App SHALL route key events through layers: PopUp (0) → Which (1) → Tab (2) → Global (3). PopUp and Which may consume events to stop propagation; Tab and Global are always called (fallback style). Adding a new layer SHALL consist of inserting a field in App and one check line in handle_key_event.

#### Scenario: PopUp blocks all layers
- **WHEN** PopUp is active
- **THEN** key events SHALL be routed to PopUp and no other layer

#### Scenario: Which consumes matching chord key
- **WHEN** Which (chord) is active and user presses a key that matches a candidate
- **THEN** the key SHALL be consumed by Which and no further routing occurs

#### Scenario: Non-matching key cancels chord and is consumed
- **WHEN** Which is active and user presses a non-matching key (e.g., `1`)
- **THEN** Which SHALL cancel the chord and return true, consuming the key (no further routing)

### Requirement: Esc cancels chord without propagation
When a chord is active and Esc is pressed, ChordHandler SHALL clear the chord state and return true (key consumed). No other layer SHALL process the Esc.

#### Scenario: Esc during chord input
- **WHEN** chord mode is active (pressed not empty)
- **AND** user presses Esc
- **THEN** chord state is cleared and the key is consumed

### Requirement: Non-matching key cancels chord and is consumed
When a chord is active and a non-matching key is pressed, ChordHandler SHALL clear the chord state and return true (key consumed). The key SHALL NOT be processed by any subsequent layer.

#### Scenario: Non-matching key during chord
- **WHEN** chord mode has candidates `[(g,g,)]` and user presses `x`
- **THEN** chord is cleared and the key is consumed (no routing to global/tab)

### Requirement: ChordHandler dispatches on exact match or single candidate
ChordHandler SHALL automatically dispatch the chord action when any candidate's key sequence length exactly matches the accumulated pressed keys, OR when only one candidate remains after filtering.

#### Scenario: Exact length match dispatches
- **WHEN** chord mode has pressed `[g, g]` and a candidate `(g g, desc)` has length 2
- **THEN** the chord action is dispatched via the callback and chord is cleared

#### Scenario: Single remaining candidate dispatches on next key
- **WHEN** only one candidate remains in the list
- **AND** user presses the next matching key
- **THEN** the chord action is dispatched and chord is cleared

### Requirement: ChordHandler initiates chord on prefix detection
When no chord is active and a key event matches the first element of one or more multi-key shortcuts (and no single-key shortcut matches that key first), ChordHandler SHALL enter chord mode with the pressed key and filtered candidates.

#### Scenario: Single-key shortcut takes priority over chord prefix
- **WHEN** a key matches both a single-key shortcut (`d` → Delete) and is a chord prefix (`d d` → SomeAction)
- **THEN** the single-key shortcut SHALL be dispatched and chord mode SHALL NOT be entered

#### Scenario: Key starts chord with multiple candidates
- **WHEN** a key `g` matches no single-key shortcut but is a prefix for `g g` and `g e`
- **THEN** chord mode enters with pressed `[g]` and candidates `[(g g,), (g e,)]`

### Requirement: Tab shortcuts are queried zero-allocation
`TuiTab::shortcuts()` SHALL return `&[(KeyCombo, &'static str)]` without heap allocation on each call. Tab implementations SHALL use static caching (`OnceLock`) for the display-formatted shortcuts.

#### Scenario: Shortcuts cached per tab type
- **WHEN** `Tab<Profile>::shortcuts()` is called multiple times
- **THEN** only the first call performs allocation; subsequent calls return a reference to the cached data
