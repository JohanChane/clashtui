# profile-update-output Specification

## Purpose
TBD - created by archiving change extract-network-resources. Update Purpose after archive.
## Requirements
### Requirement: Multi-line update results
The system SHALL produce multi-line update results when a profile update includes sub-resource downloads.

#### Scenario: Profile with no sub-resources
- **WHEN** a profile is updated and no network resources are extracted from its YAML
- **THEN** the system SHALL return a single-line result: `Updated: <profile-name>(<domain>)`

#### Scenario: Profile with successfully downloaded sub-resources
- **WHEN** a profile `my-profile` is updated and two proxy-providers are successfully downloaded
- **THEN** the system SHALL return results with the profile line first, followed by indented provider lines: `Updated: my-profile(example.com)` and `  Updated: proxy0(cdn.example.net)` and `  Updated: proxy1(cache.example.org)`

#### Scenario: Profile with mix of success and failure
- **WHEN** a profile has three sub-resources where one download fails
- **THEN** the system SHALL return results including `Not Updated: <name>(<domain>)` for the failed resource, and `Updated: <name>(<domain>)` for successful ones

### Requirement: URL privacy in output
The system SHALL display only the domain portion of URLs in update result messages, never the full URL.

#### Scenario: Simple URL
- **WHEN** a resource URL is `https://sub.example.com/path/to/config.yaml?token=secret`
- **THEN** the output SHALL show only `sub.example.com`

#### Scenario: URL without path
- **WHEN** a resource URL is `https://raw.githubusercontent.com`
- **THEN** the output SHALL show only `raw.githubusercontent.com`

#### Scenario: URL without protocol
- **WHEN** a resource URL has no recognizable protocol (no `://`)
- **THEN** the output SHALL show `Unknown domain` as a fallback

### Requirement: Update results displayed via Confirm popup
The system SHALL display update results in the existing `Confirm` popup widget through the async callback pattern.

#### Scenario: Single profile update
- **WHEN** a profile update completes in the TUI (via `u` key)
- **THEN** the system SHALL show a `Confirm` popup titled "Updated" with the multi-line result text

#### Scenario: UpdateAll results
- **WHEN** UpdateAll completes for multiple profiles
- **THEN** the system SHALL show a `Confirm` popup with title "All Updated" or "Updated (some failed)" depending on whether any profile had failures

