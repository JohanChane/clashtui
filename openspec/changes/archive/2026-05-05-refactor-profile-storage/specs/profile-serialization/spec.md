## MODIFIED Requirements

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


