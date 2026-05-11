# template-proxy-provider-availability Specification

## Purpose

Define the verification logic that ensures all proxy-provider files referenced by a template profile exist before the profile can be selected as the active core configuration.

## ADDED Requirements

### Requirement: Check proxy-provider file existence before profile selection

The system SHALL verify that all proxy-provider files for a template profile exist in the proxy-providers cache directory before allowing the profile to be selected as the active core configuration.

#### Scenario: All proxy-provider files exist

- **WHEN** a template profile references proxy providers `pvd0` and `pvd1`
- **AND** both `proxy-providers/tpl/<tpl_name>/pvd0.yaml` and `proxy-providers/tpl/<tpl_name>/pvd1.yaml` exist
- **THEN** the profile selection SHALL proceed normally

#### Scenario: Some proxy-provider files missing

- **WHEN** a template profile references proxy providers `pvd0` and `pvd1`
- **AND** `proxy-providers/tpl/<tpl_name>/pvd0.yaml` exists but `proxy-providers/tpl/<tpl_name>/pvd1.yaml` does not
- **THEN** the profile selection SHALL fail with an error message listing the missing files

#### Scenario: No proxy-provider files needed

- **WHEN** a template profile has an empty `proxy_provider_groups` (no URLs)
- **THEN** the availability check SHALL pass (no files to verify)

### Requirement: File availability check uses template file's proxy_provider_groups

The system SHALL read `clashtui.proxy_provider_groups` from the template file to determine which proxy-provider files need to exist.

#### Scenario: Check availability reads from template file

- **WHEN** `check_template_ppg_availability("common_tpl.yaml.tpl")` is called
- **THEN** the function SHALL read `templates/common_tpl.yaml` to get the proxy_provider_groups
- **AND** SHALL check existence of each `proxy-providers/tpl/common_tpl/<provider_name>.yaml`

### Requirement: Availability check for both Mihomo and sing-box

The system SHALL support proxy-provider file availability checks for both Mihomo and sing-box template profiles, using the appropriate cache directory for each core type.

#### Scenario: Mihomo availability check

- **WHEN** core type is Mihomo and `check_template_ppg_availability()` is called
- **THEN** the proxy-provider files SHALL be looked up in `<core_config_dir>/proxy-providers/tpl/<tpl_name>/`

#### Scenario: Sing-box availability check

- **WHEN** core type is SingBox and `check_template_ppg_availability()` is called
- **THEN** the proxy-provider files SHALL be looked up in `<clashtui_config_dir>/sing-box/proxy-providers/`
