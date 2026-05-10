## ADDED Requirements

### Requirement: Link nodes display target proxy type and delay

The system SHALL display the proxy type tag (e.g., `[vmess]`, `[ss]`) and delay value (e.g., `150ms`) for Link nodes in the proxy tree, using the type and delay of the proxy that the Link references.

#### Scenario: Link shows type and delay of referenced proxy

- **WHEN** the proxy tree contains a Link node that references a proxy with type `vmess` and delay `150ms`
- **THEN** the Link row SHALL display `[vmess]` in the type column and `150ms` in the delay column

#### Scenario: Link shows only type when referenced proxy has no delay

- **WHEN** the proxy tree contains a Link node that references a proxy with type `Selector` and no delay history
- **THEN** the Link row SHALL display `[Selector]` in the type column and nothing in the delay column

### Requirement: Per-node delay test via t key

Pressing the `t` key while a File or Link node is selected in the proxy tree SHALL trigger a latency test for that single proxy node and display the result in the tree.

#### Scenario: Test delay of a single leaf node

- **WHEN** user selects a File node and presses `t`
- **THEN** the system SHALL call the delay API for that node, await the response, and update the node's delay value in the tree inline within 2 seconds

#### Scenario: Test delay of a Link node

- **WHEN** user selects a Link node and presses `t`
- **THEN** the system SHALL call the delay API for the proxy that the Link references, await the response, and update the Link row's delay value inline

#### Scenario: Test delay shows loading feedback

- **WHEN** a per-node delay test is in progress
- **THEN** the system SHALL display a status message indicating which node is being tested

### Requirement: Per-group delay test via t key on Folder

Pressing the `t` key while a Folder node is selected SHALL trigger a latency test for all proxy nodes within that group and refresh the tree with updated delay values.

#### Scenario: Test delay of all nodes in a group

- **WHEN** user selects a Folder node and presses `t`
- **THEN** the system SHALL call the group delay API for that folder, wait for completion, re-fetch the full proxy list, and rebuild the tree so all child nodes show updated delay values

#### Scenario: Group test shows loading feedback

- **WHEN** a group delay test is in progress
- **THEN** the system SHALL display a status message indicating which group is being tested

### Requirement: Global delay test via a t chord

The `a t` multi-key chord SHALL trigger a latency test for ALL proxy nodes across all groups and refresh the tree with updated delay values.

#### Scenario: Test delay of all nodes globally

- **WHEN** user presses `a` then `t`
- **THEN** the system SHALL trigger delay tests for every group and standalone node, display a progress counter (e.g., "Testing 3/12"), and upon completion re-fetch the full proxy list and rebuild the tree

#### Scenario: Global test progress updates

- **WHEN** a global `a t` test is running
- **THEN** the progress counter SHALL update as each group or individual test completes, and the UI SHALL remain responsive to keyboard input throughout
