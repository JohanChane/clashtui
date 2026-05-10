# singbox-config-generation Specification

## Purpose
Define how demotui generates a native sing-box JSON configuration from its internal proxy/profile data model.

## ADDED Requirements

### Requirement: Generate singbox outbounds from profile data
The system SHALL convert profile proxy data into sing-box `outbounds[]` entries. Each proxy node SHALL be mapped to the appropriate sing-box outbound type with correct fields (tag, server, server_port, tls, transport, multiplex).

#### Scenario: Generate VLESS outbound
- **WHEN** profile data contains a VLESS node
- **THEN** the generated JSON SHALL include an outbound with `"type": "vless"`, `"tag": "<name>"`, `"server": "<addr>"`, and appropriate TLS/transport settings

#### Scenario: Generate Shadowsocks outbound
- **WHEN** profile data contains a Shadowsocks node
- **THEN** the generated JSON SHALL include an outbound with `"type": "shadowsocks"`, `"tag": "<name>"`, `"method": "<method>"`, `"password": "<password>"`

#### Scenario: Generate selector outbound group
- **WHEN** profile data contains a Selector proxy group
- **THEN** the generated JSON SHALL include an outbound with `"type": "selector"`, `"tag": "<group-name>"`, and `"outbounds": ["<child-1>", "<child-2>", ...]`

#### Scenario: Generate urltest outbound group
- **WHEN** profile data contains a URLTest proxy group
- **THEN** the generated JSON SHALL include an outbound with `"type": "urltest"`, `"tag": "<group-name>"`, `"outbounds": [...]`, and a `"url"` field

### Requirement: Generate singbox route rules
The system SHALL generate `route.rules[]` entries that direct traffic to the appropriate outbounds. At minimum, the final rule SHALL route unmatched traffic to the default outbound group.

#### Scenario: Default route rule
- **WHEN** profile is selected
- **THEN** the generated config SHALL include `"route": { "rules": [ ... ], "final": "<default-outbound>" }` or equivalent

#### Scenario: Generated config contains rule sets
- **WHEN** profile data specifies geo-based routing
- **THEN** the system MAY generate `route.rule_set[]` entries with remote URLs for geo data

### Requirement: Generate singbox DNS configuration
The system SHALL generate `dns.servers[]` entries for DNS resolution, including a FakeIP server and fallback DNS servers routed through direct or proxy outbounds.

#### Scenario: DNS servers generation
- **WHEN** config generation runs
- **THEN** the generated config SHALL include at least one DNS server with `"address": "223.5.5.5"` (or equivalent) and `"detour": "direct"`

#### Scenario: FakeIP DNS server
- **WHEN** demotui's config specifies TUN/FakeIP mode
- **THEN** the generated config SHALL include a DNS server with `"address": "fakeip"` and appropriate routing rules

### Requirement: Generate singbox inbounds
The system SHALL generate `inbounds[]` entries for TUN, mixed HTTP/SOCKS, and the `clash_api` experimental controller.

#### Scenario: TUN inbound generation
- **WHEN** demotui config has TUN enabled
- **THEN** the generated config SHALL include an inbound with `"type": "tun"`, appropriate addresses, and `"stack": "system"`

#### Scenario: clash_api inbound generation
- **WHEN** config generation runs
- **THEN** the generated config SHALL include `"experimental": { "clash_api": { "external_controller": "<addr>", "secret": "<secret>" } }`

#### Scenario: Mixed HTTP/SOCKS inbound
- **WHEN** demotui config specifies a mixed port
- **THEN** the generated config SHALL include an inbound with `"type": "mixed"` and `"listen_port": <port>`

### Requirement: Config validation before deployment
The system SHALL run `sing-box check -c <config_path>` before deploying generated config. If validation fails, the config SHALL NOT be deployed and errors SHALL be shown to the user.

#### Scenario: Validation passes
- **WHEN** `sing-box check` exits with code 0
- **THEN** the generated config SHALL be written to the sing-box config directory
- **AND** the service SHALL be reloaded

#### Scenario: Validation fails
- **WHEN** `sing-box check` exits with non-zero code and error output
- **THEN** the generated config SHALL NOT be deployed
- **AND** error details SHALL be displayed to the user via TUI
