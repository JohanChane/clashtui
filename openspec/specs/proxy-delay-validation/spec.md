# proxy-delay-validation Specification

## Purpose
TBD - created by archiving change fix-proxies-speed-test. Update Purpose after archive.
## Requirements
### Requirement: Zero-delay values are filtered from history

The system SHALL treat a delay value of `0` returned by the Mihomo delay API as a test failure and SHALL NOT add it to the proxy's delay history.

#### Scenario: Group delay test returns zero for unreachable node

- **WHEN** `GET /group/{name}/delay` returns `{"node1": 150, "node2": 0, "node3": 320}`
- **THEN** only `node1` (150) and `node3` (320) SHALL be recorded in their respective proxy histories
- **AND** `node2`'s history SHALL remain unchanged (no new entry added)

#### Scenario: Single-node delay test returns zero

- **WHEN** `GET /proxies/{name}/delay` returns `{"delay": 0}`
- **THEN** no new delay record SHALL be added to the proxy's history
- **AND** the proxy's displayed delay SHALL show "FAIL" in the tree

#### Scenario: Existing zero values in Mihomo's stored history are preserved

- **WHEN** Mihomo's `/proxies` response includes a `history` field with entries containing `"delay": 0`
- **THEN** those records SHALL remain in the deserialized proxy history (they reflect Mihomo's own state)

### Requirement: Failed delay tests display "FAIL" in the UI

When a proxy node has no valid (non-zero) delay value in its latest history, the system SHALL display "FAIL" instead of "0ms" in the proxy tree.

#### Scenario: Node with only zero-delay history entries shows FAIL

- **WHEN** a proxy's latest history record has delay `0` and is shown in the tree
- **THEN** the delay column SHALL display "FAIL" with red/dimmed styling
- **AND** the node SHALL NOT sort ahead of nodes with valid delays when sorted by delay

#### Scenario: Node with no history at all shows empty delay

- **WHEN** a proxy has never been tested and has an empty history
- **THEN** the delay column SHALL display nothing (empty string)

#### Scenario: Node with valid delay shows normal value

- **WHEN** a proxy's latest history record has delay `231`
- **THEN** the delay column SHALL display "231ms" with normal styling

### Requirement: Zero-delay nodes sort to the bottom

When the tree is sorted by delay (`s d`), nodes whose latest delay is `0` or `None` SHALL sort to the bottom of their parent group, below all nodes with valid positive delays.

#### Scenario: Sort by delay puts failed nodes last

- **WHEN** sort-by-delay is active and a group contains nodes with delays `[150, 0, 320, 0, 80]`
- **THEN** the sort order SHALL be `80, 150, 320` followed by the two failed nodes (order among failures is alphabetic)

