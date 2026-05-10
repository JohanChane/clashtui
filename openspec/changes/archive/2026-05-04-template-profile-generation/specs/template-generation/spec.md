## ADDED Requirements

### Requirement: Template proxy-provider expansion

The system SHALL expand proxy-provider entries marked with `tpl_param` into one entry per available URL. The expansion MUST preserve the relative ordering of entries from the template input.

#### Scenario: Single provider with two URLs

- **WHEN** a template has one proxy-provider entry `pvd` with `tpl_param` and two URLs are available
- **THEN** the output SHALL contain `pvd0` and `pvd1` in that order, each with the corresponding URL injected and `tpl_param` removed

#### Scenario: Mixed template and non-template providers

- **WHEN** a template has proxy-providers `[static: {...}, pvd: {tpl_param}, other: {...}]` and one URL is available
- **THEN** the output proxy-providers mapping SHALL iterate as `[static, pvd0, other]`, preserving original ordering

#### Scenario: No proxy-provider section

- **WHEN** a template has no `proxy-providers` key
- **THEN** generation SHALL fail with an error indicating missing `proxy-providers`

#### Scenario: Zero matching URLs

- **WHEN** a template has a `tpl_param` proxy-provider but zero URLs match the `clashtui.uses` filter
- **THEN** the output SHALL contain zero expanded entries for that provider (all template entries are removed); non-template entries are preserved

### Requirement: Template proxy-group expansion

The system SHALL expand proxy-group entries with `tpl_param.providers` into one group per generated proxy-provider. The expansion MUST preserve the relative ordering of groups from the template input.

#### Scenario: Single group with one provider having two URLs

- **WHEN** a template has proxy-group `Auto` with `tpl_param.providers: ["pvd"]` and provider `pvd` generates `[pvd0, pvd1]`
- **THEN** the output SHALL contain `Auto-pvd0` and `Auto-pvd1` in that order, each with `use: [pvdN]` set correctly

#### Scenario: Non-template groups preserved

- **WHEN** a template has proxy-groups `[Direct: {...}, Auto: {tpl_param.providers: ["pvd"]}, Reject: {...}]`
- **THEN** the output groups SHALL iterate as `[Direct, Auto-pvd0, (...all expanded...), Reject]`, preserving original ordering

#### Scenario: No proxy-groups section

- **WHEN** a template has no `proxy-groups` key
- **THEN** generation SHALL fail with an error indicating missing `proxy-groups`

#### Scenario: tpl_param entry missing providers key

- **WHEN** a proxy-group has `tpl_param` but no `providers` sub-key
- **THEN** generation SHALL fail with an error indicating the missing field

### Requirement: Angle-bracket placeholder substitution

The system SHALL replace `<>`-wrapped names in proxy-group `use` and `proxies` lists with the corresponding expanded names.

#### Scenario: Provider placeholder in use list

- **WHEN** a proxy-group has `use: ["DIRECT", "<pvd>"]` and provider `pvd` generates `[pvd0, pvd1]`
- **THEN** the output `use` SHALL be `["DIRECT", "pvd0", "pvd1"]`

#### Scenario: Group placeholder in proxies list

- **WHEN** a proxy-group has `proxies: ["DIRECT", "<Auto>"]` and group template `Auto` generates `[Auto-pvd0, Auto-pvd1]`
- **THEN** the output `proxies` SHALL be `["DIRECT", "Auto-pvd0", "Auto-pvd1"]`

#### Scenario: Non-placeholder values passed through

- **WHEN** a proxy-group has `proxies: ["DIRECT", "REJECT"]` with no `<>` wrapping
- **THEN** the output SHALL be `["DIRECT", "REJECT"]` unchanged

#### Scenario: Placeholder references non-existent target

- **WHEN** a proxy-group has `proxies: ["<NonExistent>"]` but no group or provider named `NonExistent` exists
- **THEN** generation SHALL fail with an error

### Requirement: clashtui marker injection

The system SHALL add a top-level `clashtui` key with null value to the generated YAML output as a type marker.

#### Scenario: Generated profile includes marker

- **WHEN** template generation completes successfully
- **THEN** the output YAML SHALL contain `clashtui: null` at the top level

### Requirement: URL path generation

The system SHALL generate `path` fields for expanded proxy-providers using the pattern `proxy-providers/tpl/{template_name}/{provider_name}.yaml`.

#### Scenario: Standard path generation

- **WHEN** template `my-tpl.yaml` generates provider `pvd0`
- **THEN** the provider SHALL have `path: proxy-providers/tpl/my-tpl/pvd0.yaml`
