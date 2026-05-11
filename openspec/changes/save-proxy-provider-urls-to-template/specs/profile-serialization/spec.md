# profile-serialization delta spec

## MODIFIED Requirements

### Requirement: Profile serialization format

The system SHALL support four profile types: `File`, `Url(String)`, `Template { template: String }`, and `Singbox`. Legacy `!Generated` entries SHALL be deserialized as `Template`. Serialization format is determined by the variant.

#### Scenario: Serialize File type

- **WHEN** a `ProfileType::File` is serialized to database
- **THEN** the YAML output SHALL be `!File`

#### Scenario: Serialize Url type

- **WHEN** a `ProfileType::Url("https://example.com")` is serialized to database
- **THEN** the YAML output SHALL be `!Url "https://example.com"`

#### Scenario: Serialize Template type

- **WHEN** a `ProfileType::Template { template: "my-tpl.yaml" }` is serialized to database
- **THEN** the YAML output SHALL be `!Template {template: my-tpl.yaml}`

#### Scenario: Deserialize legacy Generated as Template

- **WHEN** the database contains `!Generated "my-tpl.yaml"`
- **THEN** it SHALL be deserialized as `ProfileType::Template { template: "my-tpl.yaml" }` with a log warning about the migration

#### Scenario: Deserialize legacy Template with groups

- **WHEN** the database contains `!Template { template: "my-tpl.yaml", proxy_provider_groups: {pvd: {pvd0: "https://a.com"}}}`
- **THEN** it SHALL be deserialized as `ProfileType::Template { template: "my-tpl.yaml" }` with a log warning
- **AND** if the template file doesn't already have `clashtui.proxy_provider_groups`, the groups SHALL be written into the template file

### Requirement: Profile database registration

The system SHALL register the generated profile in the profile database as `ProfileType::Template { template: <template_name> }`.

#### Scenario: Registration after generation

- **WHEN** a profile is generated from template `my-tpl.yaml`
- **THEN** the database SHALL contain an entry with key `<profile_name>` and type `ProfileType::Template { template: "my-tpl.yaml" }`

#### Scenario: Re-generation updates database

- **WHEN** a template profile already exists in the database
- **THEN** re-running generation SHALL overwrite the file and update the database entry with the same template name
