# template-generation Specification

## Purpose
TBD - created by archiving change fix-template-generation. Update Purpose after archive.
## Requirements
### Requirement: Expand proxy-provider with tpl_param using template_proxy_providers URLs
The system SHALL expand each proxy-provider entry that has a `tpl_param` marker into N entries, one per URL in `template_proxy_providers`. Each generated entry SHALL have `url` set to the URL from the file and `path` set to `proxy-providers/tpl/<template_name>/<key><idx>.yaml` where `<idx>` is the zero-based index of the URL.

#### Scenario: Single provider with single URL
- **WHEN** template has proxy-provider `pvd` with `tpl_param` marker
- **AND** `template_proxy_providers` contains `https://example.com/sub1.yaml`
- **THEN** output SHALL contain `pvd0` with `url: https://example.com/sub1.yaml` and `path: proxy-providers/tpl/<tpl_name>/pvd0.yaml`
- **AND** the `tpl_param` key SHALL be removed

#### Scenario: Single provider with multiple URLs
- **WHEN** template has proxy-provider `pvd` with `tpl_param` marker
- **AND** `template_proxy_providers` contains `https://a.example.com/p1.yaml` and `https://b.example.com/p2.yaml`
- **THEN** output SHALL contain `pvd0` with `url: https://a.example.com/p1.yaml` and `path: proxy-providers/tpl/<tpl_name>/pvd0.yaml`
- **AND** output SHALL contain `pvd1` with `url: https://b.example.com/p2.yaml` and `path: proxy-providers/tpl/<tpl_name>/pvd1.yaml`

#### Scenario: Provider without tpl_param passes through unchanged
- **WHEN** template has proxy-provider `static` without `tpl_param` marker
- **THEN** output SHALL contain `static` with all its original fields unchanged

#### Scenario: Empty template_proxy_providers
- **WHEN** `template_proxy_providers` file is empty or contains only comments/blank lines
- **AND** template has proxy-provider `pvd` with `tpl_param`
- **THEN** no entry for `pvd` SHALL appear in output

### Requirement: Expand proxy-groups with tpl_param.providers
The system SHALL expand each proxy-group that has `tpl_param.providers` into one group per matching generated proxy-provider. Each expanded group SHALL be named `<group_name>-<providerN>` and have `use: [<providerN>]`.

#### Scenario: Single group with single provider expanded from one URL
- **WHEN** template has proxy-group `Auto` with `tpl_param.providers: [pvd]`
- **AND** `template_proxy_providers` contains one URL (produces `pvd0`)
- **THEN** output SHALL contain `Auto-pvd0` with `use: [pvd0]`
- **AND** `tpl_param` SHALL be removed from the group

#### Scenario: Single group with single provider expanded from multiple URLs
- **WHEN** template has proxy-group `Auto` with `tpl_param.providers: [pvd]`
- **AND** `template_proxy_providers` contains two URLs (produces `pvd0`, `pvd1`)
- **THEN** output SHALL contain `Auto-pvd0` with `use: [pvd0]`
- **AND** output SHALL contain `Auto-pvd1` with `use: [pvd1]`

#### Scenario: Group without tpl_param passes through unchanged
- **WHEN** template has proxy-group `Direct` without `tpl_param`
- **THEN** output SHALL contain `Direct` with all original fields unchanged

### Requirement: Resolve angle-bracket placeholders in group use and proxies
The system SHALL replace `<ProviderName>` placeholders in proxy-group `use` lists with all generated provider names matching that prefix, and `<GroupName>` placeholders in `proxies` lists with all generated group names matching that prefix.

#### Scenario: Provider placeholder in use list
- **WHEN** a proxy-group has `use: [<pvd>]`
- **AND** generated providers are `pvd0` and `pvd1`
- **THEN** the group's `use` SHALL be `[pvd0, pvd1]`

#### Scenario: Group placeholder in proxies list
- **WHEN** a proxy-group has `proxies: [DIRECT, <Auto>]`
- **AND** generated groups are `Auto-pvd0` and `Auto-pvd1`
- **THEN** the group's `proxies` SHALL be `[DIRECT, Auto-pvd0, Auto-pvd1]`

#### Scenario: Non-placeholder values pass through
- **WHEN** a proxy-group has `proxies: [DIRECT, REJECT]`
- **THEN** the values SHALL remain `[DIRECT, REJECT]` unchanged

### Requirement: No clashtui marker in output
The system SHALL NOT inject a `clashtui: null` key into the generated YAML output.

#### Scenario: Output has no clashtui key
- **WHEN** a template is successfully generated
- **THEN** the output YAML SHALL NOT contain a `clashtui` key

### Requirement: Template proxy-provider expansion

The system SHALL expand proxy-provider entries marked with `tpl_param` into one entry per available URL. The URLs SHALL come from the `ProfileType::Template` record, not from a `clashtui.uses` key in the template YAML. The expansion MUST preserve the relative ordering of entries from the template input.

#### Scenario: Single provider with two URLs

- **WHEN** a template has one proxy-provider entry `pvd` with `tpl_param` and two URLs are available from the profile record
- **THEN** the output SHALL contain `pvd0` and `pvd1` in that order, each with the corresponding URL injected and `tpl_param` removed

#### Scenario: Mixed template and non-template providers

- **WHEN** a template has proxy-providers `[static: {...}, pvd: {tpl_param}, other: {...}]` and one URL is available
- **THEN** the output proxy-providers mapping SHALL iterate as `[static, pvd0, other]`, preserving original ordering

#### Scenario: No proxy-provider section

- **WHEN** a template has no `proxy-providers` key
- **THEN** generation SHALL fail with an error indicating missing `proxy-providers`

#### Scenario: Zero URLs

- **WHEN** a template has a `tpl_param` proxy-provider but the profile record has zero URLs
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

### Requirement: Generated output registers as File type

The system SHALL register template-generated profiles as `ProfileType::File`, not a separate Template type.

#### Scenario: Generation creates File profile

- **WHEN** `apply_template()` completes for profile name `my-config`
- **THEN** the database SHALL contain an entry `my-config: !File`

#### Scenario: Regeneration updates existing File entry

- **WHEN** a File profile `my-config` already exists and is regenerated from template
- **THEN** the database entry SHALL remain `!File` and the YAML file SHALL be overwritten

