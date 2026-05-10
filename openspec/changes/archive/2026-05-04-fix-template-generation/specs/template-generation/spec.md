## ADDED Requirements

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
