# log-level-switching Delta Specification

## ADDED Requirements

### Requirement: Log level switching unavailable for singbox via REST API
When `core_type` is `"singbox"`, the system SHALL NOT attempt to change log level via `PATCH /configs` since sing-box's `clash_api` does not support the `log-level` patch field. The log level selector SHALL be displayed as unavailable or greyed out for sing-box.

#### Scenario: Log level selector on singbox
- **WHEN** `core_type` is `"singbox"` and user selects "Switch Log Level" from the SrvCtl tab
- **THEN** the system SHALL display a message: "Log level switching is not available for sing-box via REST API. Edit the config file directly and reload."

#### Scenario: Singbox log level displayed
- **WHEN** `core_type` is `"singbox"` and the Settings tab renders
- **THEN** the current log level SHALL be displayed (read from `GET /configs`) but marked as not switchable

### Requirement: Alternative log level change for singbox
The system SHALL inform the user that changing the sing-box log level requires editing the config file (`log.level` field) and sending SIGHUP to the sing-box process, or restarting the service.

#### Scenario: User guidance for singbox log level
- **WHEN** user presses Enter on "Switch Log Level" with singbox core
- **THEN** the system SHALL show a help message describing the manual config edit + SIGHUP approach
