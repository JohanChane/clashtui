# template-generation delta spec

## MODIFIED Requirements

### Requirement: Template proxy-provider expansion

The system SHALL expand proxy-provider entries marked with `tpl_param` into one entry per available URL. The URLs SHALL come from the template file's `clashtui.proxy_provider_groups` key, read via `read_template_ppg()`. The expansion MUST preserve the relative ordering of entries from the template input.

#### Scenario: Single provider with two URLs

- **WHEN** a template has one proxy-provider entry `pvd` with `tpl_param` and the template file's `clashtui.proxy_provider_groups` contains two URLs for `pvd`
- **THEN** the output SHALL contain `pvd0` and `pvd1` in that order, each with the corresponding URL injected and `tpl_param` removed

#### Scenario: Mixed template and non-template providers

- **WHEN** a template has proxy-providers `[static: {...}, pvd: {tpl_param}, other: {...}]` and the template file's `clashtui.proxy_provider_groups` has one URL for `pvd`
- **THEN** the output proxy-providers mapping SHALL iterate as `[static, pvd0, other]`, preserving original ordering

#### Scenario: No proxy-provider section

- **WHEN** a template has no `proxy-providers` key
- **THEN** generation SHALL fail with an error indicating missing `proxy-providers`

#### Scenario: Zero URLs

- **WHEN** a template has a `tpl_param` proxy-provider but the template file's `clashtui.proxy_provider_groups` is empty for that group
- **THEN** the output SHALL contain zero expanded entries for that provider (all template entries are removed); non-template entries are preserved

### Requirement: clashtui marker injection

The system SHALL add a top-level `clashtui` key containing `proxy_provider_groups` to the generated YAML/JSON output. The `proxy_provider_groups` value SHALL match the groups used during generation. If groups are empty, no `clashtui` key SHALL be added.

#### Scenario: Generated profile includes proxy_provider_groups

- **WHEN** template generation completes successfully with non-empty proxy_provider_groups
- **THEN** the output YAML SHALL contain `clashtui.proxy_provider_groups` at the top level with the same content as the input groups

#### Scenario: Generated profile omits clashtui when groups empty

- **WHEN** template generation completes successfully with empty proxy_provider_groups
- **THEN** the output YAML SHALL NOT contain a `clashtui` key

### Requirement: Expand proxy-provider with tpl_param using template_proxy_providers URLs

The system SHALL expand each proxy-provider entry that has a `tpl_param` marker into N entries, one per URL in the template file's `clashtui.proxy_provider_groups`. Each generated entry SHALL have `url` set to the URL from the template file and `path` set to `proxy-providers/tpl/<template_name>/<provider_name>.yaml`.

#### Scenario: Single provider with single URL

- **WHEN** template has proxy-provider `pvd` with `tpl_param` marker
- **AND** the template file's `clashtui.proxy_provider_groups` contains `pvd: {pvd0: "https://example.com/sub1.yaml"}`
- **THEN** output SHALL contain `pvd0` with `url: https://example.com/sub1.yaml` and `path: proxy-providers/tpl/<tpl_name>/pvd0.yaml`
- **AND** the `tpl_param` key SHALL be removed

#### Scenario: Single provider with multiple URLs

- **WHEN** template has proxy-provider `pvd` with `tpl_param` marker
- **AND** the template file's `clashtui.proxy_provider_groups` contains two URLs for `pvd`
- **THEN** output SHALL contain `pvd0` and `pvd1` each with the corresponding URL and path

#### Scenario: Provider without tpl_param passes through unchanged

- **WHEN** template has proxy-provider `static` without `tpl_param` marker
- **THEN** output SHALL contain `static` with all its original fields unchanged

#### Scenario: Empty template_proxy_providers

- **WHEN** the template file's `clashtui.proxy_provider_groups` is empty or absent
- **AND** template has proxy-provider `pvd` with `tpl_param`
- **THEN** no entry for `pvd` SHALL appear in output

### Requirement: Generated output registers as Template type

The system SHALL register template-generated profiles as `ProfileType::Template { template: <template_name> }`, storing only the template filename in the database.

#### Scenario: Generation creates Template profile

- **WHEN** `apply_template()` completes for profile name `my-config` with template `my-tpl.yaml`
- **THEN** the database SHALL contain an entry `my-config: !Template {template: my-tpl.yaml}`

#### Scenario: Regeneration updates existing Template entry

- **WHEN** a Template profile `my-config` already exists and is regenerated
- **THEN** the database entry SHALL remain `!Template {template: my-tpl.yaml}` and the YAML file SHALL be overwritten

### Requirement: Template generation only on explicit enter action

The system SHALL only regenerate a template profile when the user explicitly triggers generation from the template TUI tab (by pressing Enter on a template). The `update_template_profile()` function SHALL NOT call `apply_template()` — it SHALL only download proxy-provider files to the cache directory.

#### Scenario: Update does not regenerate

- **WHEN** `update_template_profile()` is called for a template profile
- **THEN** the system SHALL download proxy-provider files to the cache directory
- **AND** the system SHALL NOT regenerate the profile in `profiles/`
- **AND** the generated profile file SHALL remain unchanged

#### Scenario: Update triggers re-select when profile is current

- **WHEN** `update_template_profile()` completes successfully for a template profile
- **AND** this profile is the currently active profile
- **THEN** the system SHALL perform a select operation using the (unchanged) generated profile, so the core reloads with the newly downloaded proxy-provider files

#### Scenario: Enter template triggers regeneration

- **WHEN** the user presses Enter on a template in the TUI template tab
- **THEN** the system SHALL read `clashtui.proxy_provider_groups` from the template file
- **AND** SHALL call `apply_template()` to regenerate the profile
- **AND** SHALL write the new profile to `profiles/<name>.yaml`

## REMOVED Requirements

### Requirement: No clashtui marker in output

**Reason**: The design now requires a `clashtui.proxy_provider_groups` marker in generated output for self-description.
**Migration**: Generated profiles now include `clashtui.proxy_provider_groups` at the top level.

### Requirement: Expand proxy-groups with tpl_param.providers

**Reason**: The `expand_group_with` feature using `${PPG}` and `${PGG}` placeholders supersedes the older `tpl_param.providers` mechanism. This requirement is being removed as part of cleanup during the URL storage refactor; the functionality is covered by the `${PPG}` / `${PGG}` placeholder resolution.
**Migration**: Templates using `tpl_param.providers` should migrate to `${PPG}` / `${PGG}` syntax.
