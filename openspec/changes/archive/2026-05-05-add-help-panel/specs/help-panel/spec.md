# help-panel Specification

## Purpose

Define the behavior of the Help Panel overlay that displays all available keyboard shortcuts for the currently focused tab and global context.

## ADDED Requirements

### Requirement: Help panel toggles with ? key

The system SHALL toggle a help panel overlay when the user presses the `?` key. The `?` key SHALL be handled in the global key handler (layer 3) so it is available from any tab.

#### Scenario: Pressing ? opens help panel

- **WHEN** the help panel is NOT currently visible
- **AND** the user presses `?` (KeyCode::Char('?'))
- **THEN** the help panel becomes visible

#### Scenario: Pressing ? closes help panel

- **WHEN** the help panel IS currently visible
- **AND** the user presses `?`
- **THEN** the help panel is dismissed (hidden)

### Requirement: Help panel displays focused tab shortcuts

The help panel SHALL display all keyboard shortcuts for the currently focused tab in an upper section labeled with the tab name. Shortcuts SHALL be sourced from `tab.shortcuts()` which returns the focused pane's bindings for both single-pane and dual-pane tabs.

#### Scenario: Help panel shows Connections tab shortcuts

- **WHEN** the user opens help on the Connections tab
- **AND** the Connections tab has bindings like `j` (MoveDown), `gg` (GoTop), `dd` (Terminate), `/` (Search)
- **THEN** the upper section displays all Connections shortcuts with their key sequences and descriptions

#### Scenario: Help panel shows DualTab focused pane shortcuts

- **WHEN** the user opens help on the File tab (a DualTab)
- **AND** the focused pane is Profile (not Template)
- **THEN** the upper section displays Profile tab shortcuts only

#### Scenario: Help panel shows no tab shortcuts for Status tab

- **WHEN** the user opens help on the Status tab
- **AND** the Status tab has zero shortcuts (empty Key enum)
- **THEN** the upper section displays nothing (or a note that no tab shortcuts exist)

### Requirement: Help panel displays global shortcuts

The help panel SHALL display all global keyboard shortcuts in a lower section labeled "Global". Global shortcuts SHALL include at least: `1`–`5` (switch to tab), `Tab` (cycle to next tab), `?` (toggle help), `q` (quit).

#### Scenario: Help panel shows global shortcuts

- **WHEN** the help panel is visible
- **AND** regardless of which tab is focused
- **THEN** the lower section displays the global shortcut list

### Requirement: Help panel consumes all key events while visible

While the help panel is visible, the system SHALL consume all incoming key events and dismiss the help panel. No key events SHALL be routed to Chord, Tab, or Global layers.

#### Scenario: Any key dismisses help panel

- **WHEN** the help panel is visible
- **AND** the user presses any key (e.g., `j`, `Enter`, `Esc`, `Tab`)
- **THEN** the help panel is dismissed
- **AND** the key is consumed (not passed to lower layers)

#### Scenario: PopUp still has priority over help

- **WHEN** a PopUp is active (e.g., a confirmation dialog)
- **AND** the help panel is also visible
- **AND** the user presses a key
- **THEN** the PopUp handles the key first (PopUp is layer 0, higher priority)
- **AND** the help panel is NOT dismissed

### Requirement: Help panel renders as a centered overlay

The help panel SHALL render as a centered bordered popup overlay with a title. It SHALL use the `Clear` widget to blank the area before rendering. It SHALL have a dynamic height based on content and a fixed width of 60 columns.

#### Scenario: Help panel renders with correct layout

- **WHEN** the help panel is visible
- **THEN** a bordered block titled " Help " or similar is rendered centered on screen
- **AND** the upper section shows tab-specific shortcuts in a multi-column layout
- **AND** the lower section shows global shortcuts
- **AND** the content area behind the overlay is cleared (no visual artifacts)

#### Scenario: Help panel with many shortcuts uses columns

- **WHEN** a tab has more than 4 shortcuts
- **THEN** shortcuts are displayed in 2 columns to reduce vertical height
