## ADDED Requirements

### Requirement: Profile serialization format

The system SHALL support two profile types: `File` and `Url(String)`. Template profiles SHALL be stored as `File`. Legacy `!Template` and `!Generated` entries SHALL be deserialized as `!File`.

#### Scenario: Serialize File type

- **WHEN** a `ProfileType::File` is serialized to database
- **THEN** the YAML output SHALL be `!File`

#### Scenario: Serialize Url type

- **WHEN** a `ProfileType::Url("https://example.com")` is serialized to database
- **THEN** the YAML output SHALL be `!Url "https://example.com"`

#### Scenario: Deserialize legacy Template as File

- **WHEN** the database contains `!Template { template: "my-tpl.yaml", urls: ["https://a.com"] }`
- **THEN** it SHALL be deserialized as `ProfileType::File` with a log warning about the migration

#### Scenario: Deserialize legacy Generated as File

- **WHEN** the database contains `!Generated "my-tpl.yaml"`
- **THEN** it SHALL be deserialized as `ProfileType::File` with a log warning about the migration

### Requirement: Profile update reads from profile_yamls only

The system SHALL update profiles by reading the YAML file from `profile_yamls/<name>.yaml`, merging with basic config, and writing to the clash config path. No regeneration or re-download SHALL occur during update.

#### Scenario: Update File profile

- **WHEN** any profile of type `File` is updated
- **THEN** the system SHALL read `profile_yamls/<name>.yaml`, merge with basic config, and write to clash config path

#### Scenario: Update Url profile

- **WHEN** any profile of type `Url` is updated
- **THEN** the system SHALL read `profile_yamls/<name>.yaml`, merge with basic config, and write to clash config path
- **THEN** no re-download SHALL be triggered

## MODIFIED Requirements

### Requirement: Database persistence

The system SHALL persist the updated profile database to `clashtui.db` via `ProfileManager::to_file()` after any modification.

#### Scenario: Database saved after generation

- **WHEN** `apply_template()` completes successfully
- **THEN** the `clashtui.db` file SHALL contain the new `!File` entry
