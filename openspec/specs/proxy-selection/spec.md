# proxy-selection Specification

## Purpose
TBD - created by archiving change add-proxies-tab. Update Purpose after archive.
## Requirements
### Requirement: Select node for Selector proxy group
The system SHALL allow the user to change the selected node of a Selector-type proxy group by issuing a PUT request to `/proxies/<group-name>` with the target node name.

#### Scenario: Successful node switch
- **WHEN** user selects a Selector group and chooses a new node from its `all` list
- **AND** the Mihomo API accepts the PUT request
- **THEN** the system SHALL update the local proxy data to reflect the new selection
- **AND** the UI SHALL show the new `now` value

#### Scenario: Node switch failure
- **WHEN** the PUT request fails (network error or invalid selection)
- **THEN** the system SHALL display an error via the `tri!` macro
- **AND** the proxy tree SHALL retain the previous state

### Requirement: Present available nodes for selection
When the user triggers node selection on a Selector group, the system SHALL present the list of available nodes from the group's `all` field in a PopUp for the user to choose from.

#### Scenario: PopUp shows available nodes
- **WHEN** user presses `Enter` or `s` on a focused Selector group
- **THEN** a Choice PopUp SHALL display listing all child nodes from the group's `all` array
- **AND** the currently selected node (`now`) SHALL be pre-highlighted

#### Scenario: Cancel node selection
- **WHEN** the PopUp is open and user presses `Esc`
- **THEN** the PopUp SHALL close via `Route::Drop`
- **AND** the proxy tree SHALL remain unchanged

#### Scenario: Leaf nodes have no selection
- **WHEN** user presses `Enter` or `s` on a leaf node (e.g., Vmess, Direct)
- **THEN** no action SHALL be taken (leaf nodes cannot be changed)

### Requirement: Update proxy tree after selection
After a successful node switch, the system SHALL trigger a refresh of the proxy tree to reflect the updated state from the API.

#### Scenario: Refresh after switch
- **WHEN** a PUT /proxies/<name> completes successfully
- **THEN** the callback closure SHALL trigger a new proxy data fetch
- **AND** the tree SHALL be rebuilt with the updated `now` values

