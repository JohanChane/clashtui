# singbox-install-paths Specification

## Purpose
Define install directory structure and config directory separation for sing-box alongside mihomo, and the core-type switching mechanism in demotui config.

## ADDED Requirements

### Requirement: Core type configuration
The system SHALL support a `core_type` field in `config.yaml` that accepts `"mihomo"` or `"singbox"`. When absent, the default SHALL be `"mihomo"` for backward compatibility.

#### Scenario: Explicit mihomo core type
- **WHEN** `config.yaml` contains `core-type: mihomo`
- **THEN** demotui SHALL use mihomo-specific paths, controller URL, and service name

#### Scenario: Explicit singbox core type
- **WHEN** `config.yaml` contains `core-type: singbox`
- **THEN** demotui SHALL use sing-box-specific paths, controller URL, and service name

#### Scenario: Missing core type defaults to mihomo
- **WHEN** `config.yaml` does not contain a `core-type` field
- **THEN** demotui SHALL behave as if `core-type: mihomo`

### Requirement: Per-core binary and config paths
The system SHALL support per-core configuration for binary path, config directory, and config file path. When `core_type` is `"mihomo"`, the `basic.clash_*` fields SHALL be used. When `core_type` is `"singbox"`, the `singbox.singbox_*` fields SHALL be used.

#### Scenario: Singbox paths configured
- **WHEN** `core-type: singbox` and `singbox.singbox-bin-path: "/usr/bin/sing-box"` and `singbox.singbox-config-dir: "~/.config/clashtui/sing-box"`
- **THEN** service control operations SHALL use `/usr/bin/sing-box` as the binary
- **AND** config generation SHALL write to `~/.config/clashtui/sing-box/config.json`

#### Scenario: Singbox paths with defaults
- **WHEN** `core-type: singbox` and no `singbox` section is present in config
- **THEN** `singbox_bin_path` SHALL default to `"/usr/bin/sing-box"`
- **AND** `singbox_config_dir` SHALL default to `"~/.config/clashtui/sing-box"`

### Requirement: Install directory structure
The system SHALL support an install directory layout where mihomo and sing-box reside in separate subdirectories under `/opt/clashtui/`.

#### Scenario: Mihomo install path
- **WHEN** the system looks for the mihomo binary
- **THEN** it SHALL check `/opt/clashtui/mihomo/` for the mihomo executable and config files

#### Scenario: Singbox install path
- **WHEN** the system looks for the sing-box binary
- **THEN** it SHALL check `/opt/clashtui/sing-box/` for the sing-box executable and config files

### Requirement: Config directory structure
The system SHALL use separate config subdirectories under `~/.config/clashtui/` for each core type.

#### Scenario: Mihomo config directory
- **WHEN** `core_type` is `"mihomo"`
- **THEN** config files SHALL reside in `~/.config/clashtui/mihomo/`

#### Scenario: Singbox config directory
- **WHEN** `core_type` is `"singbox"`
- **THEN** config files SHALL reside in `~/.config/clashtui/sing-box/`

### Requirement: Per-core REST API controller URL
The system SHALL determine the REST API controller URL based on the active core type. For mihomo, the `external_controller` from `basic_clash_config.yaml` SHALL be used. For sing-box, the `external_controller` from the sing-box config SHALL be used.

#### Scenario: Singbox controller URL
- **WHEN** `core_type` is `"singbox"` and sing-box config specifies `experimental.clash_api.external_controller: "127.0.0.1:9090"`
- **THEN** all REST API calls SHALL target `http://127.0.0.1:9090`

#### Scenario: API secret per core
- **WHEN** `core_type` is `"singbox"` and sing-box config specifies a secret
- **THEN** the REST API client SHALL include `Authorization: Bearer {secret}` in requests
