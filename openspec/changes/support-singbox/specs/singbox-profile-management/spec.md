# singbox-profile-management Specification

## Purpose
Define how sing-box profiles are imported, stored, updated, and selected — accounting for sing-box's lack of proxy-provider support.

## ADDED Requirements

### Requirement: Singbox profile type
The system SHALL support a `Singbox` profile type in the profile database, distinct from `File` and `Url`. Singbox profiles SHALL be stored as JSON files in a `profile_jsons/` directory.

#### Scenario: Create singbox profile
- **WHEN** user imports a sing-box JSON config as a profile
- **THEN** the profile SHALL be stored as `ProfileType::Singbox` in the database
- **AND** the JSON file SHALL be saved to `profile_jsons/<name>.json`

#### Scenario: Singbox profile persists across restarts
- **WHEN** demotui exits and restarts
- **THEN** singbox profiles SHALL be reloaded from the database with type `ProfileType::Singbox`

### Requirement: Import singbox profile from file
The system SHALL allow importing a sing-box JSON config file as a profile. The imported file SHALL be validated as valid JSON before acceptance.

#### Scenario: Import valid singbox JSON
- **WHEN** user imports a file containing valid sing-box JSON with `outbounds` array
- **THEN** the file SHALL be copied to `profile_jsons/` and registered in the database

#### Scenario: Import invalid JSON
- **WHEN** user imports a file that is not valid JSON
- **THEN** the system SHALL reject the import with an error message

#### Scenario: Import JSON without outbounds
- **WHEN** user imports a JSON file without an `outbounds` field
- **THEN** the system SHALL warn the user but still accept the profile

### Requirement: Update singbox profile from URL
The system SHALL support updating a singbox profile by downloading the latest JSON from a configured URL. Unlike mihomo profiles, sing-box profiles SHALL NOT support proxy-provider resolution during update.

#### Scenario: Update singbox URL profile
- **WHEN** user triggers update on a `ProfileType::Singbox` with a URL source
- **THEN** the system SHALL download the JSON from the URL and replace the local file
- **AND** no proxy-provider download SHALL be attempted

#### Scenario: Update singbox file profile
- **WHEN** user triggers update on a `ProfileType::Singbox` file profile
- **THEN** the system SHALL re-read the local JSON file and regenerate config

### Requirement: No proxy-provider in singbox
The system SHALL NOT attempt to resolve proxy-providers or rule-providers for sing-box profiles. The `no_pp` flag SHALL be ignored for `ProfileType::Singbox` profiles.

#### Scenario: no_pp toggle on singbox profile
- **WHEN** user attempts to toggle `no_pp` on a singbox profile
- **THEN** the system SHALL show "Not applicable for sing-box profiles" or silently ignore

#### Scenario: Profile update for singbox skips provider resolution
- **WHEN** `update_profile()` is called for a `ProfileType::Singbox` profile
- **THEN** the function SHALL NOT call `fetch_net_resource_statuses()` or any provider download logic

### Requirement: Select singbox profile
When the user selects a singbox profile, the system SHALL generate sing-box JSON config from the profile data, validate it with `sing-box check`, and reload the sing-box service.

#### Scenario: Successful singbox profile selection
- **WHEN** user selects a singbox profile
- **THEN** the system SHALL generate `singbox_config.json` in the sing-box config directory
- **AND** validate via `sing-box check -c <path>`
- **AND** trigger service reload via SIGHUP or restart

#### Scenario: Profile validation failure
- **WHEN** `sing-box check` reports config errors
- **THEN** the system SHALL display the validation errors to the user
- **AND** the previous config SHALL remain active
