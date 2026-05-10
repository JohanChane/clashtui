## ADDED Requirements

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
