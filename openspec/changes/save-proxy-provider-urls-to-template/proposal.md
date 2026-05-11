## Why

Proxy-provider URLs are currently duplicated across `clashtui.db` and `template_proxy_providers.yaml`, creating a consistency problem: the two sources can diverge. The design doc (commit `502cc5b8f271`) resolves this by making the template file itself the single source of truth for URLs, under a `clashtui.proxy_provider_groups` key, and simplifying the database record to just the template name.

## What Changes

- **BREAKING**: `ProfileType::Template` no longer stores `proxy_provider_groups` — only `template: String`
- **BREAKING**: `template_proxy_providers.yaml` standalone file is retired; URLs move into each template file under `clashtui.proxy_provider_groups`
- New function to read `clashtui.proxy_provider_groups` from a template YAML file
- `apply_template()` and `apply_template_singbox()` read URLs from the template file, not from a standalone YAML or from `clashtui.db`
- `read_template_proxy_providers()` is refactored to read from a specific template file
- Generated profile output includes a `clashtui.proxy_provider_groups` key at the top, making profiles self-describing
- Template profile update/download reads URLs from the template file's `clashtui.proxy_provider_groups`
- Template profile selection checks that all proxy-provider files exist before using the generated profile

## Capabilities

### New Capabilities

- `template-proxy-provider-groups`: Read and write the `clashtui.proxy_provider_groups` key in template YAML files as the canonical URL storage
- `template-proxy-provider-availability`: Verify that all proxy-provider files referenced by a template exist before selecting a template-generated profile

### Modified Capabilities

- `template-generation`: URLs now come from the template file's `clashtui.proxy_provider_groups` key instead of a separate `template_proxy_providers.yaml` file or the profile database; generated output includes `clashtui.proxy_provider_groups`
- `template-profile-record`: `ProfileType::Template` drops `proxy_provider_groups` field; database stores only template filename; legacy entries with `proxy_provider_groups` are migrated on load
- `profile-serialization`: `ProfileType::Template` serialization format changes (no more `proxy_provider_groups` field); legacy `!Template` entries with groups are deserialized with migration

## Impact

- `src/config/database.rs` — `ProfileType::Template` variant, serialization/deserialization, migration from legacy `Generated`, tests
- `src/functions/file/template.rs` — `read_template_proxy_providers()`, `apply_template()`, `apply_template_singbox()`, `urls_to_groups()`
- `src/functions/file/template/version1.rs` — `gen_template()` to inject `clashtui.proxy_provider_groups` in output
- `src/functions/file/template/singbox.rs` — sing-box template generation to inject `clashtui.proxy_provider_groups` in output
- `src/functions/file/profile.rs` — `update_template_profile()` reads URLs from template file
- `src/tui/tab/files/template.rs` — `generate()` action reads URLs from template file, `_edit_providers()` opens template file instead of standalone YAML
- `src/config.rs` — path helpers for `template_proxy_providers.yaml` may be removed
- `src/config/util.rs` — `TEMPLATE_PROXY_PROVIDERS_FILE` constant may be removed
