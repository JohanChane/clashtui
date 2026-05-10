# connection-management Delta Specification

## MODIFIED Requirements

### Requirement: Close single connection
The system SHALL allow the user to close a single connection via HTTP DELETE `/connections/:id` after confirmation, targeting the active core's REST API (mihomo or sing-box).

#### Scenario: Close single connection with confirmation
- **WHEN** the user selects a connection and presses `dd` (chord: d then d)
- **THEN** the system SHALL show an AskConfirm popup asking "Terminate this connection?"
- **AND** upon confirmation (Enter/y), SHALL send DELETE `/connections/:id` and refresh the connection list

#### Scenario: Cancel close single connection
- **WHEN** the user presses `dd` then cancels the confirmation (Esc/n/q)
- **THEN** the system SHALL not send any DELETE request and keep the connection visible

#### Scenario: Close single connection failure
- **WHEN** the DELETE request fails (network error or non-empty response)
- **THEN** the system SHALL display an error message and keep the connection in the list

### Requirement: Close all connections
The system SHALL allow the user to close all connections via HTTP DELETE `/connections` after confirmation, targeting the active core's REST API.

#### Scenario: Close all connections with confirmation
- **WHEN** the user presses `ac` (chord: a then c)
- **THEN** the system SHALL show an AskConfirm popup asking "Terminate all connections?"
- **AND** upon confirmation, SHALL send DELETE `/connections` and refresh the connection list

#### Scenario: Cancel close all connections
- **WHEN** the user presses `ac` then cancels the confirmation
- **THEN** the system SHALL not send any DELETE request and keep connections visible

## ADDED Requirements

### Requirement: Handle singbox missing connection metadata
The system SHALL handle sing-box connections that lack certain metadata fields present in mihomo connections. The `ctype` field (connection type) and `nsMode` field SHALL be treated as optional and displayed as empty or "N/A" when absent.

#### Scenario: Singbox connection with missing ctype
- **WHEN** a sing-box connection has no `type` field in its metadata
- **THEN** the connection SHALL still be displayed in the connections list
- **AND** the type column SHALL show an empty string or "N/A"

#### Scenario: Connection display with empty metadata fields
- **WHEN** a sing-box connection has empty `host` and `processPath`
- **THEN** the connection SHALL fall back to displaying `destinationIP:destinationPort` as the host
