# profile-serialization Specification

## Purpose
TBD - created by archiving change per-profile-no-pp. Update Purpose after archive.
## Requirements
### Requirement: Per-profile no_pp flag in database

The system SHALL store a `no_pp: bool` flag for each profile in the database (`profiles.yaml`). The flag controls whether net resources (proxy-providers, rule-providers) are removed and embedded during profile update.

#### Scenario: New profile defaults to false

- **WHEN** a profile is created (via `add`, `import`, or `apply_template`)
- **THEN** the `no_pp` flag SHALL be `false`

#### Scenario: Toggle no_pp on existing profile

- **WHEN** the user toggles `no_pp` for a profile via the TUI keybinding
- **THEN** the flag SHALL be flipped and persisted to the database immediately

#### Scenario: Update uses stored no_pp

- **WHEN** `update_profile()` is called for a profile with `no_pp: true`
- **THEN** the function SHALL download and embed all net resources (proxy-providers, rule-providers) into the profile YAML

#### Scenario: Update skips embedding when no_pp is false

- **WHEN** `update_profile()` is called for a profile with `no_pp: false`
- **THEN** the function SHALL NOT embed net resources

### Requirement: Backward-compatible database deserialization

The system SHALL deserialize both old-format and new-format profile entries in `profiles.yaml`.

#### Scenario: Old format — bare ProfileType

- **WHEN** the database contains `pf1: File` or `pf2: !Url "https://example.com"`
- **THEN** the profile SHALL deserialize with `no_pp: false`

#### Scenario: New format — structured ProfileData

- **WHEN** the database contains `pf1: { dtype: File, no_pp: true }`
- **THEN** the profile SHALL deserialize with `no_pp: true`

#### Scenario: New format with missing no_pp field

- **WHEN** the database contains `pf1: { dtype: File }` (no `no_pp` key)
- **THEN** the profile SHALL deserialize with `no_pp: false`

### Requirement: New format serialization on save

The system SHALL always serialize profiles in the new structured format when saving the database.

#### Scenario: Database save writes new format

- **WHEN** `ProfileManager::to_file()` is called
- **THEN** each profile entry SHALL be written as `{ dtype: <ProfileType>, no_pp: <bool> }`

### Requirement: Generated profile serialization to file

The system SHALL serialize the expanded template YAML to `profile_yamls/<name>.yaml` using `serde_yml::to_writer`.

#### Scenario: Successful write

- **WHEN** template generation completes and produces a valid `serde_yml::Mapping`
- **THEN** a file is created at `profile_yamls/<name>.yaml` containing the YAML
- **THEN** the file SHALL be valid YAML parseable by `serde_yml::from_reader`

#### Scenario: Write failure

- **WHEN** the target path is not writable (e.g., permissions, disk full)
- **THEN** generation SHALL fail with an I/O error

### Requirement: Profile database registration

The system SHALL register the generated profile in the profile database as `ProfileType::Template { template: <template_name>, urls: <url_list> }`.

#### Scenario: Registration after generation

- **WHEN** a profile is generated from template `my-tpl.yaml` with URLs `["https://a.com"]`
- **THEN** the database SHALL contain an entry with key `<profile_name>` and type `ProfileType::Template { template: "my-tpl.yaml", urls: ["https://a.com"] }`

#### Scenario: Re-generation updates database

- **WHEN** a template profile already exists in the database with different URLs
- **THEN** re-running generation SHALL overwrite the file and update the database entry with the new URL list

### Requirement: Database persistence

The system SHALL persist the updated profile database to `clashtui.db` via `ProfileManager::to_file()` after generation.

#### Scenario: Database saved after generation

- **WHEN** `apply_template()` completes successfully
- **THEN** the `clashtui.db` file SHALL contain the new `ProfileType::Template` entry
- **THEN** subsequent calls to `get_all()` SHALL include the new entry

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

