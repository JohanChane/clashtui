# mode-switching Delta Specification

## ADDED Requirements

### Requirement: Mode switching for singbox core
The system SHALL support mode switching for sing-box via `PATCH /configs` with `{"mode": "<mode>"}`, the only config field sing-box supports patching via REST API. The available modes SHALL be the same as mihomo: rule, direct, global.

#### Scenario: Switch mode on singbox
- **WHEN** `core_type` is `"singbox"` and user selects "Direct" in the mode selector
- **THEN** the system SHALL issue `PATCH /configs` to the sing-box controller with body `{"mode": "direct"}`
- **AND** on success, display a success message

#### Scenario: Mode switch failure on singbox
- **WHEN** the `PATCH /configs` request to sing-box fails
- **THEN** the system SHALL show an error popup

### Requirement: Limited settings patch for singbox
When `core_type` is `"singbox"`, the system SHALL only allow `mode` changes via `PATCH /configs`. All other settings (tun, allow_lan, ipv6, log_level, etc.) SHALL NOT be patchable via REST API for sing-box.

#### Scenario: Singbox settings tab display
- **WHEN** `core_type` is `"singbox"` and user views the Settings tab
- **THEN** non-patchable settings SHALL be displayed but marked as read-only or greyed out
