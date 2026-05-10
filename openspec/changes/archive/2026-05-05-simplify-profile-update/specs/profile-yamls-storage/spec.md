## ADDED Requirements

### Requirement: profile_yamls directory creation

The system SHALL create a `profile_yamls/` directory under the config root directory during `init_config()`.

#### Scenario: Directory creation on first run

- **WHEN** the config directory is initialized for the first time
- **THEN** a `profile_yamls/` subdirectory SHALL be created alongside `templates/`

### Requirement: profile_yamls is sole profile directory

The system SHALL use `profile_yamls/` as the only directory for all profile YAML files. The `profiles/` directory SHALL NOT be used.

#### Scenario: Profile loading reads from profile_yamls

- **WHEN** `load_local_profile()` is called for any profile type
- **THEN** the file path SHALL be `profile_yamls/<name>.yaml`

#### Scenario: Generated profiles written to profile_yamls

- **WHEN** `apply_template()` completes for profile name `my-config`
- **THEN** a file SHALL be created at `profile_yamls/my-config.yaml`

#### Scenario: Imported profiles stored in profile_yamls

- **WHEN** a file is imported via `I` key
- **THEN** it SHALL be copied to `profile_yamls/<name>.yaml`
