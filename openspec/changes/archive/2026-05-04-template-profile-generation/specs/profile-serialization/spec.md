## ADDED Requirements

### Requirement: Generated profile serialization to file

The system SHALL serialize the expanded template YAML to `profiles/<name>.clashtui_generated` using `serde_yml::to_writer`.

#### Scenario: Successful write

- **WHEN** template generation completes and produces a valid `serde_yml::Mapping`
- **THEN** a file is created at `profiles/<name>.clashtui_generated` containing the YAML
- **THEN** the file SHALL be valid YAML parseable by `serde_yml::from_reader`

#### Scenario: Write failure

- **WHEN** the target path is not writable (e.g., permissions, disk full)
- **THEN** generation SHALL fail with an I/O error

### Requirement: Profile database registration

The system SHALL register the generated profile in the profile database as `ProfileType::Generated(template_name)`.

#### Scenario: Registration after generation

- **WHEN** a profile is generated from template `my-tpl.yaml`
- **THEN** the database SHALL contain an entry with key `my-tpl.yaml.clashtui_generated` and type `ProfileType::Generated("my-tpl.yaml")`

#### Scenario: Re-generation overwrites

- **WHEN** a profile from template `my-tpl.yaml` already exists in the database
- **THEN** re-running generation SHALL overwrite the file and update (or re-insert) the database entry

### Requirement: Database persistence

The system SHALL persist the updated profile database to `clashtui.db` via `ProfileManager::to_file()` after generation.

#### Scenario: Database saved after generation

- **WHEN** `apply_template()` completes successfully
- **THEN** the `clashtui.db` file SHALL contain the new `ProfileType::Generated` entry
- **THEN** subsequent calls to `get_all()` SHALL include the new entry
