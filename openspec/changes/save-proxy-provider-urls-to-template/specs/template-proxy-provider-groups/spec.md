# template-proxy-provider-groups Specification

## Purpose

Define the `clashtui.proxy_provider_groups` key in template YAML files as the canonical storage for proxy-provider URLs, and provide functions to read and write this data.

## ADDED Requirements

### Requirement: Template file contains proxy_provider_groups

The system SHALL support a top-level `clashtui` key in template YAML files containing a `proxy_provider_groups` mapping. The mapping SHALL have the structure `{group_name: {provider_name: url}}`.

#### Scenario: Template file with proxy_provider_groups

- **WHEN** a template file contains `clashtui: {proxy_provider_groups: {pvd: {pvd0: "https://example.com/sub.yaml"}}}`
- **AND** `read_template_ppg("my-tpl.yaml")` is called
- **THEN** the returned `ProxyProviderGroups` SHALL contain group `pvd` with provider `pvd0` mapping to `https://example.com/sub.yaml`

#### Scenario: Template file without clashtui key

- **WHEN** a template file has no `clashtui` top-level key
- **AND** `read_template_ppg("no-ppg.yaml")` is called
- **THEN** the function SHALL return an empty `ProxyProviderGroups`

#### Scenario: Template file without proxy_provider_groups sub-key

- **WHEN** a template file has `clashtui: {}` (no `proxy_provider_groups` key)
- **AND** `read_template_ppg("empty.yaml")` is called
- **THEN** the function SHALL return an empty `ProxyProviderGroups`

### Requirement: read_template_ppg reads from templates directory

The system SHALL provide a function `read_template_ppg(template_name: &str) -> anyhow::Result<ProxyProviderGroups>` that reads the template file from `templates/<template_name>`, parses the `clashtui.proxy_provider_groups` key, and returns the groups.

#### Scenario: Template file not found

- **WHEN** `read_template_ppg("nonexistent.yaml")` is called
- **AND** the file does not exist in the templates directory
- **THEN** the function SHALL return an error

#### Scenario: Template file with invalid YAML

- **WHEN** `read_template_ppg("malformed.yaml")` is called
- **AND** the file contains invalid YAML syntax
- **THEN** the function SHALL return an error

### Requirement: Generated profile output includes proxy_provider_groups

The system SHALL inject a top-level `clashtui` key with `proxy_provider_groups` into the generated profile YAML/JSON output when `apply_template()` or `apply_template_singbox()` is called with non-empty groups.

#### Scenario: Mihomo generated profile includes clashtui key

- **WHEN** `apply_template("my-tpl.yaml", "my-profile", groups)` is called with non-empty groups
- **THEN** the output YAML at `profiles/my-profile.yaml` SHALL contain `clashtui.proxy_provider_groups` with the same content as `groups`

#### Scenario: Empty groups omit clashtui key from output

- **WHEN** `apply_template("my-tpl.yaml", "my-profile", empty_groups)` is called
- **THEN** the output YAML SHALL NOT contain a `clashtui.proxy_provider_groups` key

#### Scenario: Sing-box generated profile includes clashtui key

- **WHEN** `apply_template_singbox("my-tpl.json", "my-profile", groups, false, false)` is called with non-empty groups
- **THEN** the output JSON at `profiles/my-profile.json` SHALL contain `clashtui.proxy_provider_groups` with the same content as `groups`

### Requirement: write_template_ppg merges proxy_provider_groups into template file

The system SHALL provide a function `write_template_ppg(template_name: &str, groups: &ProxyProviderGroups) -> anyhow::Result<()>` that writes or updates the `clashtui.proxy_provider_groups` key in the template file. Other keys in the file SHALL be preserved unchanged.

#### Scenario: Add proxy_provider_groups to template without existing clashtui key

- **WHEN** `write_template_ppg("tpl.yaml", groups)` is called on a template file without a `clashtui` key
- **THEN** the file SHALL gain a top-level `clashtui.proxy_provider_groups` key with the provided groups
- **AND** all existing keys in the file SHALL be preserved

#### Scenario: Update existing proxy_provider_groups

- **WHEN** `write_template_ppg("tpl.yaml", new_groups)` is called on a template file that already has `clashtui.proxy_provider_groups`
- **THEN** the `proxy_provider_groups` value SHALL be replaced with `new_groups`
- **AND** all other keys SHALL be preserved

### Requirement: read proxy_provider_groups from generated profile

The system SHALL provide a function `read_profile_ppg(profile_name: &str) -> anyhow::Result<ProxyProviderGroups>` that reads `clashtui.proxy_provider_groups` from a generated profile file in `profiles/<profile_name>.yaml`. This is used during the update flow to determine which URLs to download.

#### Scenario: Generated profile has proxy_provider_groups

- **WHEN** `read_profile_ppg("my-config.tpl")` is called
- **AND** `profiles/my-config.tpl.yaml` contains `clashtui.proxy_provider_groups`
- **THEN** the returned groups SHALL match the file's content

#### Scenario: Generated profile has no proxy_provider_groups

- **WHEN** `read_profile_ppg("my-config.tpl")` is called
- **AND** `profiles/my-config.tpl.yaml` has no `clashtui` key
- **THEN** the function SHALL return an empty `ProxyProviderGroups`
