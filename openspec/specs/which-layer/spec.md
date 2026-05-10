# which-layer Specification

## Purpose
TBD - created by archiving change add-multi-key-shortcuts-and-which-panel. Update Purpose after archive.
## Requirements
### Requirement: Which layer is the first key routing stage

The Which layer SHALL be the first stage in `App::handle_key_event`, running before PopUp, App-global, and Tab routing. When active (showing candidates), it SHALL consume all key events and prevent them from reaching lower layers.

#### Scenario: Key reaches Which before PopUp
- **WHEN** a key event arrives and a PopUp is also visible
- **THEN** the Which layer receives the key first; if it consumes it (match or candidate filtering), the PopUp does NOT receive it

#### Scenario: Unknown key falls through to Tab
- **WHEN** a key event arrives, Which is inactive, and no shortcut matches the key
- **THEN** the key falls through to PopUp → App → Tab for normal handling

### Requirement: Single-key shortcuts dispatch transparently

When the Which layer is inactive and a key press matches a single-key shortcut (sequence length 1), the action SHALL be dispatched immediately without showing the which panel.

#### Scenario: Single-key action dispatched without UI
- **WHEN** user presses `i` and `i` is a single-key shortcut mapped to `Key::Action(Add)`
- **THEN** `dispatch_shortcut` is called, the action executes, and no which panel is displayed

### Requirement: Multi-key prefix activates which panel

When the Which layer is inactive and a key press matches one or more chord prefixes (sequence length > 1, first key matches), the which panel SHALL be activated showing all candidate completions.

#### Scenario: Prefix key opens which panel
- **WHEN** user presses `g` and shortcuts include `(g,g) → GoTop` and `(g,e) → GoEnd` but no single-key `g` shortcut
- **THEN** the which panel opens showing candidates `g → Go to top` and `e → Go to end`

#### Scenario: Single-key takes priority over chord with same key
- **WHEN** user presses `g` and `g` is BOTH a single-key shortcut AND a chord prefix
- **THEN** the single-key action executes immediately; the chord is NOT activated

### Requirement: Subsequent keys filter candidates

When the which panel is active, each subsequent key press SHALL filter the candidate list to only those where the sequence at the current position matches the pressed key.

#### Scenario: Second key narrows candidates
- **WHEN** which panel shows `[g→GoTop, h→GoHome, d→GoDownloads]` and user presses `g`
- **THEN** `GoTop` action dispatches and the which panel closes (last remaining candidate auto-executes)

#### Scenario: Non-matching key closes which
- **WHEN** which panel shows candidates `[g→GoTop, h→GoHome]` and user presses `x`
- **THEN** candidates become empty and the which panel closes without dispatching any action

### Requirement: Auto-dispatch on single remaining candidate

When the candidate list is reduced to exactly 1 entry after filtering, the action SHALL be dispatched immediately without waiting for further input.

#### Scenario: Auto-dispatch on single candidate
- **WHEN** which panel shows `[g→GoTop]` as the only candidate (after filtering) and user presses `g`
- **THEN** the `GoTop` action dispatches and the which panel closes

#### Scenario: Auto-dispatch when prefix exactly matches a chord length
- **WHEN** which panel shows candidates `[g→GoTop, h→GoHome]` where `GoTop`'s sequence length equals pressed keys count, and user presses `g`
- **THEN** `GoTop` dispatches immediately

### Requirement: Esc dismisses which panel

When the which panel is active, pressing Escape SHALL close the panel without dispatching any action.

#### Scenario: Esc closes which
- **WHEN** which panel is active showing candidates
- **THEN** pressing `Esc` closes the panel; no action is dispatched; subsequent keys are routed normally

### Requirement: Which panel renders as overlay

The which panel SHALL render as a floating bordered box overlaying the tab content. It SHALL display candidates as `key → description` pairs in 1-2 columns.

#### Scenario: Few candidates in 1 column
- **WHEN** which panel has 3 candidates
- **THEN** candidates are rendered in a single column with the format `key  description`

#### Scenario: Many candidates in 2 columns
- **WHEN** which panel has 5+ candidates and terminal is wide enough
- **THEN** candidates are rendered in 2 columns

#### Scenario: Panel auto-closes after dispatch
- **WHEN** a candidate action is dispatched
- **THEN** the which panel is not rendered in the next frame

