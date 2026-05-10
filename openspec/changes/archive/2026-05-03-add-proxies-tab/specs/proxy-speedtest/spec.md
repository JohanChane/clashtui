## ADDED Requirements

### Requirement: Test delay of a single proxy
The system SHALL measure the delay of a single proxy by calling `GET /proxies/<name>/delay` with configurable URL and timeout parameters, then update the local delay cache.

#### Scenario: Successful single speed test
- **WHEN** user presses `t` on a focused proxy node
- **THEN** the system SHALL spawn an async task that calls `GET /proxies/<name>/delay?url=<test-url>&timeout=5000`
- **AND** on success, update the node's displayed delay in the tree

#### Scenario: Single speed test failure
- **WHEN** the delay API call fails or times out
- **THEN** the node SHALL display `-` or `timeout` instead of a delay value
- **AND** the error SHALL NOT cause the entire tree to fail to render

#### Scenario: Speed test in progress indicator
- **WHEN** a speed test is in progress for a node
- **THEN** the node SHALL display a rotating animation (`-/|\`) next to its name
- **AND** the animation SHALL stop when the test completes

### Requirement: Batch speed test for proxy group
The system SHALL support batch speed testing all nodes in a proxy group by calling `GET /group/<name>/delay`, then refreshing the proxy tree to display updated delays.

#### Scenario: Successful batch speed test
- **WHEN** user presses `T` (shift-t) on a focused proxy group
- **THEN** the system SHALL spawn an async task that calls `GET /group/<name>/delay?url=<test-url>&timeout=5000`
- **AND** on success, trigger a full proxy tree refresh to load updated delays

#### Scenario: Batch speed test on leaf node is no-op
- **WHEN** user presses `T` on a leaf node (no children)
- **THEN** no action SHALL be taken

### Requirement: Display delay history on demand
The system SHALL display delay history from the proxy's `history` field when the user explicitly requests it.

#### Scenario: Show delay history
- **WHEN** user presses `h` on a node that has delay history
- **THEN** a PopUp SHALL display the recent delay history records with timestamps
