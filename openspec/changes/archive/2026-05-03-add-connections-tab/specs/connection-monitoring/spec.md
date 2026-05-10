## ADDED Requirements

### Requirement: Fetch active connections

The system SHALL fetch the list of active connections from Mihomo via HTTP GET `/connections` and display them in a table.

#### Scenario: Successful fetch
- **WHEN** the ConnectionsTab is active
- **THEN** the system polls `/connections` every 1 second and populates the connection table with the response

#### Scenario: Fetch failure
- **WHEN** the Mihomo API returns an error or is unreachable
- **THEN** the system SHALL display an error message and retain the previously fetched connection data

### Requirement: Display connection table

The system SHALL render connections as a ratatui Table with columns: Host, Rule, Chains, Download, Upload, DL Speed, UL Speed.

#### Scenario: Table rendering
- **WHEN** connection data is available
- **THEN** the system SHALL render a header row with column titles and data rows with connection information, using the active theme's style for selection highlighting

#### Scenario: Empty state
- **WHEN** there are zero active connections
- **THEN** the system SHALL display an empty table with the header row and a message indicating no active connections

### Requirement: Navigate connection list

The system SHALL support keyboard navigation through the connection table.

#### Scenario: Move selection down
- **WHEN** the user presses `j` or `↓`
- **THEN** the system SHALL move the selection highlight one row down, wrapping or clamping at the end

#### Scenario: Move selection up
- **WHEN** the user presses `k` or `↑`
- **THEN** the system SHALL move the selection highlight one row up, wrapping or clamping at the top

#### Scenario: Jump to first
- **WHEN** the user presses `gg` (chord: g then g)
- **THEN** the system SHALL move the selection to the first row

#### Scenario: Jump to last
- **WHEN** the user presses `G`
- **THEN** the system SHALL move the selection to the last row

### Requirement: Tab registration

The system SHALL register ConnectionsTab as tab index 3, accessible via digit key `4`.

#### Scenario: Tab switch
- **WHEN** the user presses `4`
- **THEN** the system SHALL switch to the ConnectionsTab and show the connection table

### Requirement: Sort connections

The system SHALL support sorting connections by download speed, upload speed, and resetting to default order.

#### Scenario: Sort by download speed
- **WHEN** the user presses `sd` (chord: s then d)
- **THEN** the system SHALL sort connections by download speed in descending order and update the table
- **AND** the DL Speed column header SHALL display a `▼` marker

#### Scenario: Sort by upload speed
- **WHEN** the user presses `su` (chord: s then u)
- **THEN** the system SHALL sort connections by upload speed in descending order and update the table
- **AND** the UL Speed column header SHALL display a `▼` marker

#### Scenario: Reset sort order
- **WHEN** the user presses `sr` (chord: s then r)
- **THEN** the system SHALL restore connections to their original API-returned order
- **AND** no column header SHALL display sort markers
