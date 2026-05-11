# template-profile-record delta spec

## MODIFIED Requirements

### Requirement: Template profile record structure

The system SHALL support a `ProfileType::Template` variant with a single field `template: String` (template file name in `templates/`). The template's proxy-provider URLs SHALL be read from the template file's `clashtui.proxy_provider_groups` key at point of use, not stored in the database.

#### Scenario: Create template profile

- **WHEN** a user creates a profile with template name `my-tpl.yaml`
- **THEN** the profile database SHALL contain an entry with `ProfileType::Template { template: "my-tpl.yaml".into() }`

#### Scenario: Serialize template profile to database

- **WHEN** a `ProfileType::Template` entry is persisted to `clashtui.db`
- **THEN** the serialized YAML SHALL contain only the `template` field

#### Scenario: Template profile database YAML format

- **WHEN** the database is saved
- **THEN** a Template entry SHALL be serialized as `!Template { template: "my-tpl.yaml" }`

### Requirement: Profile database migration from Generated to Template

The system SHALL deserialize legacy `ProfileType::Generated(String)` entries as `ProfileType::Template { template: <name> }` with a log warning about the migration.

#### Scenario: Load old database with Generated entries

- **WHEN** `clashtui.db` contains `!Generated my-tpl.yaml`
- **THEN** the deserialized profile type SHALL be `ProfileType::Template { template: "my-tpl.yaml" }`

### Requirement: Legacy Template entries with proxy_provider_groups are migrated

The system SHALL deserialize legacy `!Template { template, proxy_provider_groups }` entries. On deserialization, if `proxy_provider_groups` is non-empty, the system SHALL write the groups into the template file's `clashtui.proxy_provider_groups` key (if not already present) and log a migration warning. The in-memory representation SHALL discard the groups.

#### Scenario: Load old Template entry with non-empty groups

- **WHEN** `clashtui.db` contains `!Template { template: "tpl.yaml", proxy_provider_groups: {pvd: {pvd0: "https://a.com"}}}`
- **AND** the template file `templates/tpl.yaml` does not already have a `clashtui.proxy_provider_groups` key
- **THEN** the groups SHALL be written into `templates/tpl.yaml` under `clashtui.proxy_provider_groups`
- **AND** the in-memory `ProfileType` SHALL be `Template { template: "tpl.yaml" }`
- **AND** a migration warning SHALL be logged

#### Scenario: Load old Template entry where template file already has groups

- **WHEN** `clashtui.db` contains a legacy `!Template { template, proxy_provider_groups }` entry
- **AND** the template file already has a `clashtui.proxy_provider_groups` key
- **THEN** the database groups SHALL NOT overwrite the template file
- **AND** a migration warning SHALL be logged noting that groups were preserved from the template file

### Requirement: Template profile stores URLs in template file only

The system SHALL NOT store proxy-provider URLs in `ProfileType::Template`. URLs for template expansion SHALL come from the template file's `clashtui.proxy_provider_groups` key, read via `read_template_ppg()`.

#### Scenario: URL sourcing from template file during generation

- **WHEN** `apply_template()` is called for a profile with `ProfileType::Template { template: "my-tpl" }`
- **THEN** `read_template_ppg("my-tpl")` SHALL be called to obtain the URL groups for expansion

#### Scenario: URL sourcing from generated profile during update

- **WHEN** `update_template_profile()` is called for a template profile
- **THEN** `read_profile_ppg("my-config.tpl")` SHALL be called to obtain URLs from the generated profile file
- **AND** only proxy-provider downloads SHALL be performed (no regeneration)

## REMOVED Requirements

### Requirement: Template profile with empty URL list

**Reason**: URLs are no longer stored in the database, so there is no "empty URL list" concept at the database level. An empty proxy_provider_groups in the template file is still valid.
**Migration**: No action needed. Templates with no `clashtui.proxy_provider_groups` key are treated as having empty groups.
