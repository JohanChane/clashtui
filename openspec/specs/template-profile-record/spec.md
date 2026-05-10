# template-profile-record Specification

## Purpose
TBD - created by archiving change refactor-profile-storage. Update Purpose after archive.
## Requirements
### Requirement: Template profile record structure

The system SHALL support a `ProfileType::Template` variant with fields `template: String` (template file name in `templates/`) and `urls: Vec<String>` (URL strings for proxy-provider expansion).

#### Scenario: Create template profile

- **WHEN** a user creates a profile with template name `my-tpl.yaml` and URLs `["https://sub1.example.com", "https://sub2.example.com"]`
- **THEN** the profile database SHALL contain an entry with `ProfileType::Template { template: "my-tpl.yaml".into(), urls: vec!["https://sub1.example.com".into(), "https://sub2.example.com".into()] }`

#### Scenario: Template profile with empty URL list

- **WHEN** a template profile is created with an empty URL list `urls: []`
- **THEN** the entry SHALL be valid; template expansion SHALL produce zero expanded proxy-providers

#### Scenario: Serialize template profile to database

- **WHEN** a `ProfileType::Template` entry is persisted to `clashtui.db`
- **THEN** the serialized YAML SHALL contain `template` and `urls` fields

### Requirement: Profile database migration from Generated to Template

The system SHALL deserialize legacy `ProfileType::Generated(String)` entries as `ProfileType::Template { template: <name>, urls: [] }` with a log warning about the empty URL list.

#### Scenario: Load old database with Generated entries

- **WHEN** `clashtui.db` contains `!Generated my-tpl.yaml`
- **THEN** the deserialized profile type SHALL be `ProfileType::Template { template: "my-tpl.yaml", urls: [] }`

### Requirement: Template profile stores URLs directly

The system SHALL NOT read a `clashtui.uses` key from template YAML. URLs for template expansion SHALL come from the `ProfileType::Template` record's `urls` field.

#### Scenario: URL sourcing from profile record

- **WHEN** `apply_template()` is called for a profile with `ProfileType::Template { template: "my-tpl", urls: ["https://a.com", "https://b.com"] }`
- **THEN** `gen_template()` SHALL receive the URL list `["https://a.com", "https://b.com"]` directly from the profile record

