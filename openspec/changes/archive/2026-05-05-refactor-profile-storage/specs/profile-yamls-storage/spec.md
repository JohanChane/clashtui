## MODIFIED Requirements

### Requirement: profile_yamls directory creation

The system SHALL create a `profile_yamls/` directory under the config root directory during `init_config()`.

#### Scenario: Directory creation on first run

- **WHEN** the config directory is initialized for the first time
- **THEN** a `profile_yamls/` subdirectory SHALL be created alongside `profiles/` and `templates/`

#### Scenario: Directory exists on subsequent runs

- **WHEN** the config directory already contains a `profile_yamls/` directory
- **THEN** `init_config()` SHALL NOT fail; the existing directory SHALL be reused

## ADDED Requirements

### Requirement: Generated profiles written to profile_yamls

The system SHALL write template-generated clash YAML output to `profile_yamls/<profile_name>.yaml`.

#### Scenario: Successful generation writes to profile_yamls

- **WHEN** `apply_template()` completes successfully for a profile named `my-config`
- **THEN** a file SHALL be created at `profile_yamls/my-config.yaml` containing the expanded template YAML

#### Scenario: Regeneration overwrites existing file

- **WHEN** `profile_yamls/my-config.yaml` already exists
- **THEN** re-running generation SHALL overwrite the file with new content

### Requirement: profile_yamls path constant

The system SHALL define a path constant `PROFILE_YAMLS_DIR` or equivalent that resolves to `{DATA_DIR}/profile_yamls`.

#### Scenario: Path resolution

- **WHEN** code calls `profile_yamls_path()`
- **THEN** it SHALL return `config::data_dir().join("profile_yamls")`
