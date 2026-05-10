# which-panel-rendering Specification

## Purpose
TBD - created by archiving change redesign-multi-key-framework. Update Purpose after archive.
## Requirements
### Requirement: Which panel renders when chord is active
App SHALL render the Which panel overlay when ChordHandler is active (pressed not empty). The panel SHALL be centered on screen, display remaining keys and descriptions for each candidate, and use a bordered block with "Which?" title.

#### Scenario: Panel shown during chord input
- **WHEN** ChordHandler is active (pressed not empty)
- **THEN** App renders a centered Which panel on top of tab content

#### Scenario: Panel not shown when no chord
- **WHEN** ChordHandler is NOT active
- **THEN** no Which panel is rendered

### Requirement: Panel displays remaining keystrokes per candidate
Each candidate SHALL display only the untyped portion of the key sequence (from `pressed.len()` onward), followed by the description text.

#### Scenario: Remaining keys shown
- **WHEN** chord mode has pressed `[g]` and a candidate `(g g, "Go to top")`
- **THEN** the panel displays `g  Go to top` (only the second `g`)

### Requirement: Column layout adapts to candidate count
The panel SHALL use 1 column when there are 4 or fewer candidates, and 2 columns when there are 5 or more candidates.

#### Scenario: 1 column for up to 4 candidates
- **WHEN** chord mode has 4 candidates
- **THEN** the Which panel renders in a single-column layout at 40 chars wide

#### Scenario: 2 columns for 5+ candidates
- **WHEN** chord mode has 5 or more candidates
- **THEN** the Which panel renders in a 2-column layout at 70 chars wide

### Requirement: Panel uses bordered block with Clear widget
The panel SHALL have a bordered block with "Which?" title, left-aligned. A `Clear` widget SHALL fill the panel area to mask underlying tab content.

#### Scenario: Visual presentation
- **WHEN** Which panel renders
- **THEN** a Clear widget occupies the panel rect, a bordered "Which?" block encloses candidate text lines

### Requirement: ChordHandler rendering is decoupled from App
ChordHandler SHALL expose its state for rendering via public fields (`pressed`, `candidates`). App's `render_which` SHALL read these fields to produce the panel, without ChordHandler knowing rendering details.

#### Scenario: ChordHandler has no render method
- **WHEN** ChordHandler is active
- **THEN** App reads `chord.pressed.len()` and `chord.candidates` to construct the Which panel UI

