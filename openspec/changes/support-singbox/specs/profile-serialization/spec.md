# profile-serialization Delta Specification

## ADDED Requirements

### Requirement: Singbox profile type in database
The system SHALL support `ProfileType::Singbox` as a profile type in addition to `File` and `Url`. `ProfileType::Singbox` profiles SHALL be stored in the `profile_jsons/` directory as JSON files.

#### Scenario: Serialize Singbox type
- **WHEN** a `ProfileType::Singbox` is serialized to database
- **THEN** the YAML output SHALL be `!Singbox`

#### Scenario: Deserialize Singbox type
- **WHEN** the database contains `myprofile: !Singbox`
- **THEN** it SHALL be deserialized as `ProfileType::Singbox`

#### Scenario: Singbox profile stored as JSON
- **WHEN** a singbox profile is saved to disk
- **THEN** the file SHALL be written to `profile_jsons/<name>.json` as valid JSON

### Requirement: no_pp ignored for Singbox profiles
The system SHALL ignore the `no_pp` flag for `ProfileType::Singbox` profiles. sing-box does not support proxy-provider, so there is nothing to embed or strip.

#### Scenario: Singbox profile with no_pp flag
- **WHEN** a singbox profile has `no_pp: true` in the database
- **THEN** profile update SHALL NOT attempt to download or embed any providers
- **AND** the profile update SHALL proceed as if `no_pp: false`

## MODIFIED Requirements

### Requirement: Profile serialization format
The system SHALL support three profile types: `File`, `Url(String)`, and `Singbox`. Template profiles SHALL be stored as `File`. Legacy `!Template` and `!Generated` entries SHALL be deserialized as `!File`.

#### Scenario: Serialize File type
- **WHEN** a `ProfileType::File` is serialized to database
- **THEN** the YAML output SHALL be `!File`

#### Scenario: Serialize Url type
- **WHEN** a `ProfileType::Url("https://example.com")` is serialized to database
- **THEN** the YAML output SHALL be `!Url "https://example.com"`

#### Scenario: Serialize Singbox type
- **WHEN** a `ProfileType::Singbox` is serialized to database
- **THEN** the YAML output SHALL be `!Singbox`

#### Scenario: Deserialize legacy Template as File
- **WHEN** the database contains `!Template { template: "my-tpl.yaml", urls: ["https://a.com"] }`
- **THEN** it SHALL be deserialized as `ProfileType::File` with a log warning about the migration

#### Scenario: Deserialize legacy Generated as File
- **WHEN** the database contains `!Generated "my-tpl.yaml"`
- **THEN** it SHALL be deserialized as `ProfileType::File` with a log warning about the migration
