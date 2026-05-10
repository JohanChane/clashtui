# proxy-selection Delta Specification

## MODIFIED Requirements

### Requirement: Select node for Selector proxy group
The system SHALL allow the user to change the selected node of a Selector-type proxy group by issuing a PUT request to `/proxies/<group-name>` with the target node name, targeting the REST API of the active core (mihomo or sing-box).

#### Scenario: Successful node switch
- **WHEN** user selects a Selector group and chooses a new node from its `all` list
- **AND** the core's REST API accepts the PUT request
- **THEN** the system SHALL update the local proxy data to reflect the new selection
- **AND** the UI SHALL show the new `now` value

#### Scenario: Node switch on singbox
- **WHEN** core type is `"singbox"` and user selects a new node for a Selector group
- **THEN** the system SHALL issue `PUT /proxies/<group-name>` to the sing-box controller URL
- **AND** on success, the proxy tree SHALL refresh showing the updated selection

#### Scenario: Node switch failure
- **WHEN** the PUT request fails (network error or invalid selection)
- **THEN** the system SHALL display an error via the `tri!` macro
- **AND** the proxy tree SHALL retain the previous state

## ADDED Requirements

### Requirement: Proxy delay testing for singbox
The system SHALL support delay testing for sing-box proxies using the same `GET /proxies/{name}/delay` endpoint, with the singbox controller URL.

#### Scenario: Delay test on singbox selector
- **WHEN** core type is `"singbox"` and user triggers delay test on a Selector group
- **THEN** the system SHALL issue `GET /proxies/{name}/delay?url=...&timeout=...` to the sing-box controller
- **AND** display the delay result in milliseconds

#### Scenario: Delay test failure on singbox
- **WHEN** the delay test request to sing-box fails or times out
- **THEN** the proxy SHALL show "FAIL" as its delay value
