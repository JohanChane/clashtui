## Context

`gen_template()` currently strips `tpl_param` from proxy-providers/proxy-groups but does NOT expand them using URLs from `template_proxy_providers`. The file `template_proxy_providers` (at `<config_dir>/templates/template_proxy_providers`) contains one subscription URL per line (with `#` comments and blank lines). Clashtui uses these URLs to create one proxy-provider entry per URL. Demotui currently ignores them.

Template-generated profiles are now `ProfileType::File` — they must be self-contained YAML files. Proxy-provider `url` + `path` fields must be set at generation time so clash can download providers later. No runtime template regeneration happens.

An unrelated `clashtui: null` marker is inserted into output — this serves no purpose and should be removed.

## Goals / Non-Goals

**Goals:**
- `gen_template()` reads `template_proxy_providers` and expands each `tpl_param`-marked provider into N copies (one per URL)
- Each generated provider gets `url` from the file and `path` = `proxy-providers/tpl/<tpl_name>/<key><idx>.yaml`
- Proxy-groups with `tpl_param.providers` expand into one group per generated provider, named `<group>-<providerN>`
- `<AngleBracket>` placeholders in group `use`/`proxies` resolve to expanded names
- Remove `clashtui: null` injection
- Existing `template_proxy_providers` file format works unchanged

**Non-Goals:**
- No URL downloading — clash handles that at runtime
- No change to `ProfileType` or database schema
- No change to `apply_template()` registration logic (already `ProfileType::File`)
- No change to `update_profile()` or `update_profile_without_pp()`

## Decisions

### 1. Pass `template_name` into `gen_template()`

**Decision:** Change signature from `gen_template(tpl: Mapping)` to `gen_template(tpl: Mapping, template_name: &str)`.

**Rationale:** Needs template name (without extension) for `path` generation: `proxy-providers/tpl/<tpl_name>/<key><idx>.yaml`. `apply_template()` already has the template name available.

**Alternative:** Extract name from template YAML content. Rejected — template name isn't stored in YAML, and the caller already knows it.

### 2. Read `template_proxy_providers` inside `gen_template()`

**Decision:** Read the file `TEMPLATE_PATH.parent().join("template_proxy_providers")` inside `gen_template()` using lazy-loading. Skip if file missing (no URLs → no expansion — proxy-provider entries with `tpl_param` get zero outputs).

**Rationale:** Clashtui reads this file within `crt_yaml_with_template()`. Keeps concerns together.

**Alternative:** Pass URLs as parameter. Rejected — adds coupling to callers; only gen_template needs this data.

### 3. Path format for generated providers

**Decision:** `proxy-providers/tpl/<template_name>/<pp_key><idx>.yaml` — same as clashtui.

**Rationale:** Compatible with clash directory layout expectations. Template name scopes providers to avoid collisions.

### 4. Error handling for missing `template_proxy_providers`

**Decision:** If the file doesn't exist or is empty, log a warning and treat as empty URL list. Providers with `tpl_param` get no entries; groups referencing them get no entries either. This matches clashtui behavior.

**Rationale:** Graceful degradation — a template with no external URLs is still valid (just no imported proxies).

## Risks / Trade-offs

- **Testdata churn**: All existing `*_output.yaml` files need updating → Use snapshot-style approach, regenerate expected outputs after implementation
- **Missing `template_proxy_providers` file in tests**: Tests pass `gen_template()` directly without creating real filesystem → use feature flag or an internal `gen_template_with_urls(tpl, template_name, urls: &[String])` for testability, with the public fn wrapping it

## Open Questions

- None
