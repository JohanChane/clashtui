## Why

When a profile is updated (downloaded from a subscription URL), its YAML content often references additional network resources — such as proxy-providers and rule-providers with their own `url` and `path` fields. Currently demotui only downloads the top-level profile file and ignores these embedded resources, leaving them stale. Users must manually update each provider via other means.

## What Changes

- Add a `NetworkResource` struct representing a discovered network resource (name, url, target path, section type)
- Add a `NetResourceExtract` trait with a single method to extract all network resources from parsed YAML
- Implement the trait for `serde_yml::Mapping` to walk `proxy-providers` and `rule-providers` sections
- Integrate extraction into `update_profile()`: after downloading the profile YAML, parse it for sub-resources and download each one
- Extend the update result display to show per-resource status with URL domain only (not full URLs, for security)
- Omit GitHub/Gitee/GitLab token-aware URL handling for now (the existing `ProfileType::Github`/`GitLab` variants handle those at the profile level)

## Capabilities

### New Capabilities
- `network-resource-extraction`: Extract network resource references (name, url, path, section) from YAML configuration content
- `profile-update-output`: Display multi-resource update results with URL privacy (domain-only, not raw URLs)

### Modified Capabilities
<!-- None — this is purely additive -->

## Impact

- Affected code: `src/functions/file/profile.rs` (update flow), new file `src/functions/file/net_resource.rs`
- No API changes, no dependency additions, no breaking changes
- The existing `ProfileType::Url` update path gains sub-resource download capability
- `ProfileType::Generated` may also benefit if templates contain proxy/rule providers with URLs
