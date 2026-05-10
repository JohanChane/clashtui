## Context

demotui already has a template-to-profile generation system in `src/functions/file/template/version1.rs` (`gen_template()`). This function takes a parsed template YAML as `serde_yml::Mapping` and a list of `(profile_name, url)` pairs sourced from the profile database via `clashtui.uses`. It expands `tpl_param`-marked entries in `proxy-providers` and `proxy-groups`, resolves `<>` placeholder references, adds a `clashtui: null` marker, and returns the fully expanded mapping.

The caller (`apply_template()` in `template.rs`) serializes the result to `profiles/<name>.clashtui_generated` and registers it as `ProfileType::Generated` in the database.

This system is structurally correct but lacks:
- Unit tests for the core generation logic
- Documentation for the template format and workflow
- A test data directory with sample inputs and expected outputs
- Explicit verification of ordering guarantees

## Goals / Non-Goals

**Goals:**
- Verify and harden `gen_template()` to guarantee that output YAML preserves the section ordering of the input template (already structurally correct via `serde_yml::Mapping`/`Sequence`)
- Write comprehensive unit tests for `gen_template()` covering normal flow, empty subscription filter, no `tpl_param` entries, `<>` placeholder expansion, and error paths
- Create test data YAML files in `src/functions/file/template/testdata/`
- Write user-facing documentation at `docs/profile_template.md`
- Ensure `apply_template()` correctly wires `clashtui.uses` → `gen_template()` → file write → database registration

**Non-Goals:**
- Changing the `clashtui.uses` mechanism to use a flat file like clashtui's `template_proxy_providers` (the database-backed approach is different but equally valid)
- Adding GitHub/Gitee/GitLab token handling to template providers (profile-level tokens suffice)
- Changing the `Profile` / `ProfileType` / `ProfileManager` types
- Adding new template versions (only version 1 is supported)
- Modifying the TUI tab (`src/tui/tab/files/template.rs`) — it already works with `apply_template()`

## Decisions

### 1. Keep `gen_template()` as-is, add tests around it

The existing `gen_template()` in `version1.rs:202-414` is already a faithful port of clashtui's `crt_yaml_with_template()`. Both use:
- `serde_yml::Mapping` (index-preserving) for proxy-providers
- `serde_yml::Sequence` (Vec, order-preserving) for proxy-groups
- `HashMap<String, Vec<String>>` for name tracking (pp_names, pg_names)
- Clone-and-mutate pattern for template entries

The function does not need refactoring — it needs test coverage to lock in its behavior. Any robustness fixes (e.g., better error messages, handling `<>` to non-existent names) are minor additions within the existing structure.

**Alternative considered**: Rewrite to match clashtui's `Cow` / `to_mut()` pattern. Rejected because the current clone-based approach is simpler and correct (templates are small files, cloning is cheap).

### 2. Test data in `src/functions/file/template/testdata/`

Following the pattern used by the `extract-network-resources` change (`src/functions/file/testdata/net_resource_test.yaml`), test data lives alongside the source:

```
src/functions/file/template/testdata/
  simple_tpl.yaml           # minimal template with one provider, one group, tpl_param
  multi_provider_tpl.yaml   # template with 2 proxy-providers, 2 proxy-groups
  no_tpl_param_tpl.yaml     # template with zero tpl_param entries (passthrough)
  simple_tpl_output.yaml    # expected output for simple_tpl when given 2 URLs
  empty_uses_output.yaml    # expected output when no URLs match (empty uses → no expansion)
```

Tests load both the template and the expected output, run `gen_template()`, and assert `serde_yml::to_value(out) == serde_yml::to_value(expected)`.

### 3. Ordering guarantee is structural, not code-enforced

The preservation of ordering relies on:
- `serde_yml::Mapping` internally uses `indexmap::IndexMap` (preserves insertion order)
- Iteration over `pp_mapping` (line 233) and `pg_value` (line 287) is in insertion order
- Non-template entries are pushed/inserted first, template expansions follow in their original positions

This is documented in the code via comments and in `docs/profile_template.md`, but not enforced by explicit ordering tests. The test strategy is:
- Input template: `[proxy-groups: Select, Auto(tpl_param), Direct]`
- Expected output: `[proxy-groups: Select, Auto-pvd0, Auto-pvd1, Direct]`
- Any reordering would fail the `serde_yml::to_value` equality check

### 4. URL sourcing via `clashtui.uses` vs flat file

demotui's URL source differs from clashtui:

| Aspect | clashtui | demotui |
|--------|----------|---------|
| URL storage | `templates/template_proxy_providers` text file | Profile database (profiles with `ProfileType::Url`) |
| Selection | All URLs always used | Filtered by `clashtui.uses` list of profile names |
| Multi-profile | One file for all templates | Per-template via `clashtui.uses` |

This is by design and will not be changed. The `clashtui.uses` approach is more flexible (different templates can reference different profiles). Documentation will explain this clearly.

**Alternative considered**: Add support for a flat file like clashtui. Rejected because the database approach is equally valid and avoids introducing a second URL source mechanism.

### 5. Documentation format

`docs/profile_template.md` follows the structured format:
1. Overview — what templates are and why
2. Template YAML format — `tpl_param` markers, `<>` substitution, `clashtui.uses`
3. Proxy-provider template entries — how `tpl_param` works, URL injection, path generation
4. Proxy-group template entries — `tpl_param.providers`, name generation (`{group}-{providerN}`)
5. `<>` placeholder expansion — `<provider>`, `<Group>` substitution rules
6. Workflow — create template → configure uses → generate → select profile → deploy
7. Example — complete walkthrough with sample YAML

## Risks / Trade-offs

- **Test data maintenance**: Template and expected-output YAML pairs must be kept in sync. → Mitigation: Use descriptive names and minimal examples. Document the relationship in testdata/README or comments.
- **Empty uses → empty output**: When `clashtui.uses` filters out all profiles, the generated profile has no proxy-provider expansions but still has the `clashtui` marker. This is valid but may confuse users. → Mitigation: Document this behavior and test it.
- **`<>` to missing target**: If a proxy-group references `<NonExistent>` in `proxies` or `use`, `gen_template()` currently uses `.with_context()` which propagates an error. → Mitigation: The test suite verifies that valid references succeed; invalid references are user error caught at generation time.
