## ADDED Requirements

### Requirement: Cursor defaults to first item on initial load

The system SHALL set the cursor (selected index) to the first item when a tab's data is loaded for the first time.

#### Scenario: Profile tab initial cursor

- **WHEN** the Profile tab loads its profile list for the first time
- **THEN** the cursor SHALL be positioned on the first item (index 0)

#### Scenario: Template tab initial cursor

- **WHEN** the Template tab loads its template list for the first time
- **THEN** the cursor SHALL be positioned on the first item (index 0)

#### Scenario: Proxies tab initial cursor

- **WHEN** the Proxies tab loads its proxy tree for the first time
- **THEN** the cursor SHALL be positioned on the first tree node

#### Scenario: Connections tab initial cursor

- **WHEN** the Connections tab loads its connection list for the first time
- **THEN** the cursor SHALL be positioned on the first connection row

### Requirement: Cursor position preserved across tab switches

The system SHALL preserve the cursor position (selected row) when the user switches away from a tab and returns to it later.

#### Scenario: Profile cursor preserved after switching to Connections

- **WHEN** the user scrolls to row 3 in the Profile tab
- **AND** switches to the Connections tab (via number key)
- **AND** switches back to the Profile tab
- **THEN** the cursor SHALL still be at row 3

#### Scenario: Template cursor preserved after switching

- **WHEN** the user scrolls to row 2 in the Template tab
- **AND** switches to another tab and back
- **THEN** the cursor SHALL still be at row 2

#### Scenario: Connections cursor preserved after switching

- **WHEN** the user scrolls to row 5 in Connections tab
- **AND** switches to another tab and back
- **THEN** the cursor SHALL still be at row 5

### Requirement: Cursor validity after content changes

The system SHALL clamp the cursor index to a valid range after any operation that modifies the list content, to prevent an out-of-bounds selected index.

#### Scenario: Cursor clamped after list shrinks

- **WHEN** the profile list has 5 items and the cursor is at index 4
- **AND** the last profile is deleted (list now has 4 items)
- **THEN** the cursor SHALL be clamped to index 3 (the new last item)

#### Scenario: Cursor unchanged when list grows

- **WHEN** the profile list has 3 items and the cursor is at index 1
- **AND** a new profile is added (list now has 4 items)
- **THEN** the cursor SHALL remain at index 1 (no change needed)

#### Scenario: Cursor reset to first when list becomes empty

- **WHEN** the last profile is deleted (list becomes empty)
- **THEN** the cursor selection SHALL be set to None (no selection)

#### Scenario: Cursor reset after sync removes items

- **WHEN** a background sync reduces the item count below the current cursor index
- **THEN** the cursor SHALL be clamped to the last valid index
