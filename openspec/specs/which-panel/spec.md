# which-panel Specification

## Purpose
TBD - created by archiving change chord-keybinding-system. Update Purpose after archive.
## Requirements
### Requirement: Which panel activates on multi-key prefix match
When the Router matches a key that is the first key of one or more Chords with `on.len() > 1`, the system SHALL activate the Which panel. The Which panel SHALL display all candidate Chords that start with the pressed key.

#### Scenario: Which activates with multiple candidates
- **WHEN** user presses `g` and the active layer has Chords `on = [g, g]` (scroll top) and `on = [g, e]` (scroll end)
- **THEN** the Which panel appears showing both candidates: `g → scroll top` and `e → scroll end`

#### Scenario: Which does not activate for single-key chords
- **WHEN** user presses `j` and the active layer has only a single-key Chord `on = [j]`
- **THEN** the Which panel does NOT activate; the command executes immediately

### Requirement: Which panel progressively filters candidates
As the user continues pressing keys while Which is active, the system SHALL filter the candidate list to only Chords whose `on` sequence continues to match the accumulated keys. The `times` counter SHALL track how many keys have been matched.

#### Scenario: Progressive filtering reduces candidates
- **WHEN** Which is active with candidates `[g, g]` and `[g, e]`, and user presses `g`
- **THEN** only the `[g, g]` candidate remains, and it auto-executes

#### Scenario: Non-matching key exits Which without action
- **WHEN** Which is active with candidates `[g, g]` and `[g, e]`, and user presses `x`
- **THEN** the Which panel closes (no match), no command executes, and the `x` key falls through to subsequent routing

#### Scenario: Only one candidate left auto-executes
- **WHEN** after filtering, exactly one candidate remains in the Which panel
- **THEN** that candidate's commands execute immediately without requiring further input

#### Scenario: Exact sequence match auto-executes
- **WHEN** a candidate's `on.len()` equals the current `times` counter (exact match)
- **THEN** that candidate's commands execute immediately

### Requirement: Which panel renders candidate chords with visual hierarchy
The Which panel SHALL render each candidate as a line containing: padding, the already-matched prefix key, the remaining unmatched keys, a separator character, and the chord description. The already-matched prefix SHALL be highlighted distinctly from remaining keys.

#### Scenario: Which panel visual layout
- **WHEN** Which is active with `times = 1` and candidate Chord `on = [g, e]` with description "scroll to end"
- **THEN** the rendered line shows `g` highlighted, `e` in rest style, separator, then "scroll to end"

### Requirement: Which panel exits on Escape
Pressing `Escape` while Which is active SHALL close the Which panel and reset its state without executing any command.

#### Scenario: Esc cancels multi-key input
- **WHEN** Which is active and user presses `Escape`
- **THEN** the Which panel closes, `times` resets to 0, candidates are cleared, and no command is executed

### Requirement: Which panel is independent of PopUp
The Which panel SHALL NOT be part of the PopUp message queue. It SHALL be managed as a standalone state at the `App` level. While Which is active, key events SHALL go to Which first; non-matching keys SHALL fall through to subsequent routing (PopUp, App, Tab layers).

#### Scenario: Which and PopUp coexist correctly
- **WHEN** a PopUp is displayed (e.g. Confirm) and Which is NOT active
- **THEN** keys go to PopUp as before (no change to PopUp behavior)

#### Scenario: Which takes priority over PopUp
- **WHEN** Which is active and a PopUp is also queued
- **THEN** matching keys are consumed by Which; non-matching keys fall through to PopUp

