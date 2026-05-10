## ADDED Requirements

### Requirement: Generated output registers as File type

The system SHALL register template-generated profiles as `ProfileType::File`, not a separate Template type.

#### Scenario: Generation creates File profile

- **WHEN** `apply_template()` completes for profile name `my-config`
- **THEN** the database SHALL contain an entry `my-config: !File`

#### Scenario: Regeneration updates existing File entry

- **WHEN** a File profile `my-config` already exists and is regenerated from template
- **THEN** the database entry SHALL remain `!File` and the YAML file SHALL be overwritten

## MODIFIED Requirements

### Requirement: Template proxy-provider expansion

The system SHALL strip `tpl_param` markers from proxy-provider entries and pass all other fields through unchanged. Each `tpl_param` proxy-provider SHALL generate exactly one output entry with the same name, keeping its own `url`, `path`, and all other fields from the template. The expansion MUST preserve the relative ordering of entries from the template input.

#### Scenario: Single provider preserves all fields

- **WHEN** a template has one proxy-provider entry `pvd` with `tpl_param`, `type: http`, `url: https://example.com/sub.yaml`, and `interval: 3600`
- **THEN** the output SHALL contain `pvd` with `type: http`, `url: https://example.com/sub.yaml`, `interval: 3600`, and `tpl_param` removed

#### Scenario: Mixed template and non-template providers

- **WHEN** a template has proxy-providers `[pvd: {tpl_param, url: "https://a.com"}, static: {type: http, url: "https://static.com"}]`
- **THEN** the output proxy-providers mapping SHALL contain `[pvd: {url: "https://a.com", ...}, static: {type: http, url: "https://static.com"}]`, preserving original ordering

#### Scenario: No proxy-provider section

- **WHEN** a template has no `proxy-providers` key
- **THEN** generation SHALL fail with an error indicating missing `proxy-providers`

### Requirement: Template proxy-group expansion

The system SHALL expand proxy-group entries with `tpl_param.providers` into one group per matching proxy-provider. The expansion MUST preserve the relative ordering of groups from the template input.

#### Scenario: Single group with one provider

- **WHEN** a template has proxy-group `Auto` with `tpl_param.providers: ["pvd"]` and provider `pvd` exists in proxy-providers
- **THEN** the output SHALL contain `Auto-pvd` with `use: [pvd]`

#### Scenario: Non-template groups preserved

- **WHEN** a template has proxy-groups `[Direct: {...}, Auto: {tpl_param.providers: ["pvd"]}, Reject: {...}]`
- **THEN** the output groups SHALL iterate as `[Direct, Auto-pvd, Reject]`, preserving original ordering

#### Scenario: No proxy-groups section

- **WHEN** a template has no `proxy-groups` key
- **THEN** generation SHALL fail with an error indicating missing `proxy-groups`

#### Scenario: tpl_param entry missing providers key

- **WHEN** a proxy-group has `tpl_param` but no `providers` sub-key
- **THEN** generation SHALL fail with an error indicating the missing field

#### Scenario: No matching providers

- **WHEN** `tpl_param.providers` references a name that has no matching proxy-provider in pp_names
- **THEN** the group template SHALL generate zero entries and be silently skipped

### Requirement: Angle-bracket placeholder substitution

The system SHALL replace `<>`-wrapped names in proxy-group `use` and `proxies` lists with the corresponding expanded names.

#### Scenario: Provider placeholder in use list

- **WHEN** a proxy-group has `use: ["DIRECT", "<pvd>"]` and provider `pvd` maps to entry `["pvd"]` in pp_names
- **THEN** the output `use` SHALL be `["DIRECT", "pvd"]`

#### Scenario: Group placeholder in proxies list

- **WHEN** a proxy-group has `proxies: ["DIRECT", "<Auto>"]` and group template `Auto` generates `[Auto-pvd]`
- **THEN** the output `proxies` SHALL be `["DIRECT", "Auto-pvd"]`

#### Scenario: Placeholder references non-existent target

- **WHEN** a proxy-group has `proxies: ["<NonExistent>"]` but no group or provider named `NonExistent` exists
- **THEN** generation SHALL fail with an error


