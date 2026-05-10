## ADDED Requirements

### Requirement: Import YAML file by path

The system SHALL support importing a local clash YAML configuration by filesystem path, copying it into `profile_yamls/` and registering it in the profile database.

#### Scenario: Successful file import

- **WHEN** a user imports `/home/user/my-clash-config.yaml` with profile name `my-config`
- **THEN** the file SHALL be copied to `profile_yamls/my-config.yaml`
- **THEN** the profile database SHALL contain an entry for `my-config` with type `ProfileType::File`

#### Scenario: Source file not found

- **WHEN** the source path does not exist
- **THEN** the import SHALL fail with a file-not-found error

#### Scenario: Source file is not valid YAML

- **WHEN** the source file exists but is not parseable as YAML
- **THEN** the import SHALL fail with a YAML parse error

#### Scenario: Target name already exists in profile_yamls

- **WHEN** `profile_yamls/my-config.yaml` already exists and a new file import uses the same name `my-config`
- **THEN** the system SHALL either fail with a conflict error or prompt the user to overwrite

### Requirement: Imported file is clash-ready

The system SHALL NOT modify or process the imported YAML content. It SHALL be a byte-for-byte copy of the source file.

#### Scenario: Content preservation

- **WHEN** a file is imported
- **THEN** the content of `profile_yamls/<name>.yaml` SHALL be identical to the source file

### Requirement: TUI import action

The system SHALL provide a TUI action in the profile tab to trigger file path import, prompting the user for a source file path and a profile name.

#### Scenario: User triggers import from TUI

- **WHEN** the user presses the import key binding in the profile tab
- **THEN** a PopUp SHALL appear prompting for the source file path and profile name
- **THEN** upon confirmation, the file SHALL be imported
