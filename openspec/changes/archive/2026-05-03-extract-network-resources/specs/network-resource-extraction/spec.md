# network-resource-extraction Specification

## Purpose
Define the capability to discover network resource references (URLs with target paths) within YAML configuration content, enabling automatic download of proxy-provider and rule-provider resources referenced in clash profiles.

## ADDED Requirements

### Requirement: Extract proxy-provider resources from YAML
The system SHALL extract network resources from `proxy-providers` sections in parsed YAML mapping content.

#### Scenario: Single proxy-provider with url and path
- **WHEN** parsed YAML contains `proxy-providers: { my-provider: { type: http, url: "https://cdn.example.com/proxies.yaml", path: "./proxy-providers/my-provider.yaml" } }`
- **THEN** the system SHALL return one NetResource with name `my-provider`, url `https://cdn.example.com/proxies.yaml`, path `./proxy-providers/my-provider.yaml`, and section `ProxyProvider`

#### Scenario: Proxy-provider as string value (not mapping)
- **WHEN** parsed YAML contains `proxy-providers: { my-provider: { type: http, url: "https://example.com/proxy.yaml", path: "./pp/px.yaml" } }`
- **AND** the provider value is a YAML mapping
- **THEN** the system SHALL still extract url and path from the mapping

#### Scenario: Proxy-provider without url field
- **WHEN** a proxy-provider entry has a `path` but no `url` field
- **THEN** the system SHALL skip that entry and not include it in results

#### Scenario: Proxy-provider without path field
- **WHEN** a proxy-provider entry has a `url` but no `path` field
- **THEN** the system SHALL skip that entry and not include it in results

### Requirement: Extract rule-provider resources from YAML
The system SHALL extract network resources from `rule-providers` sections in parsed YAML mapping content.

#### Scenario: Single rule-provider with url and path
- **WHEN** parsed YAML contains `rule-providers: { my-rules: { type: http, behavior: classical, url: "https://rules.example.com/list.yaml", path: "./rule-providers/my-rules.yaml" } }`
- **THEN** the system SHALL return one NetResource with name `my-rules`, section `RuleProvider`

#### Scenario: Multiple rule-providers
- **WHEN** parsed YAML contains two rule-provider entries, both with valid `url` and `path`
- **THEN** the system SHALL return two NetResources, both with section `RuleProvider`

### Requirement: Filter extraction by section type
The system SHALL allow callers to specify which section types to extract resources from.

#### Scenario: Extract only proxy-providers
- **WHEN** the caller requests extraction with sections `[ProxyProvider]` only
- **THEN** the system SHALL return only resources from `proxy-providers`, ignoring any `rule-providers`

#### Scenario: Extract both section types
- **WHEN** the caller requests extraction with sections `[ProxyProvider, RuleProvider]`
- **THEN** the system SHALL return resources from both `proxy-providers` and `rule-providers`

#### Scenario: Empty section filter
- **WHEN** the caller requests extraction with an empty section list
- **THEN** the system SHALL return an empty result set

### Requirement: Handle YAML without provider sections
The system SHALL gracefully handle YAML content that has no provider sections.

#### Scenario: No proxy-providers or rule-providers
- **WHEN** parsed YAML has no `proxy-providers` or `rule-providers` keys
- **THEN** the system SHALL return an empty result set without error

#### Scenario: Provider section is not a mapping
- **WHEN** `proxy-providers` value is a scalar or sequence (not a mapping)
- **THEN** the system SHALL skip that section and return an empty result set

### Requirement: Download network resource to target path
The system SHALL download each extracted network resource's URL and write the content to its target path within the clash config directory.

#### Scenario: Successful download
- **WHEN** a NetResource has url `https://example.com/pp.yaml` and path `proxy-providers/pp.yaml`
- **THEN** the system SHALL HTTP GET the URL via proxy, validate the response is valid YAML containing proxies or proxy-providers, and write it to `<clash_cfg_dir>/proxy-providers/pp.yaml`

#### Scenario: Download failure
- **WHEN** an HTTP GET for a NetResource URL fails (timeout, 404, connection error)
- **THEN** the system SHALL record the failure (with the domain name, not the full URL) and continue processing remaining resources

#### Scenario: Download parent directory does not exist
- **WHEN** the target path's parent directory does not exist (e.g., `proxy-providers/some/sub/dir/pp.yaml`)
- **THEN** the system SHALL create all parent directories before writing the file

### Requirement: No token injection for sub-resources
The system SHALL NOT apply GitHub/Gitee/GitLab token-based authentication to sub-resource downloads.

#### Scenario: Generic URL download
- **WHEN** a NetResource URL is a plain HTTPS URL (e.g., `https://cdn.example.com/data.yaml`)
- **THEN** the system SHALL download it using the basic profile download function without any auth token injection
