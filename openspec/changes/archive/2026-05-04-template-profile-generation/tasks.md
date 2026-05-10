## 1. Test data setup

- [x] 1.1 Create `src/functions/file/template/testdata/` directory
- [x] 1.2 Create `simple_tpl.yaml` — minimal template: one proxy-provider with `tpl_param`, one proxy-group with `tpl_param.providers`, one passthrough group, one passthrough provider, no `<>` placeholders
- [x] 1.3 Create `simple_tpl_output.yaml` — expected output for `simple_tpl.yaml` with 2 URLs (`https://example.com/sub1.yaml`, `https://example.com/sub2.yaml`)
- [x] 1.4 Create `multi_provider_tpl.yaml` — template with 2 `tpl_param` proxy-providers (`pvd` and `pvd2`), 2 template groups cross-referencing both providers, plus `<>` placeholders in a passthrough group
- [x] 1.5 Create `multi_provider_tpl_output.yaml` — expected output for `multi_provider_tpl.yaml` with 2 URLs
- [x] 1.6 Create `no_tpl_param_tpl.yaml` — template with zero `tpl_param` entries (all passthrough); verifies identity expansion
- [x] 1.7 Create `no_tpl_param_tpl_output.yaml` — expected output for `no_tpl_param_tpl.yaml` (should be identical except for added `clashtui: null`)
- [x] 1.8 Create `empty_uses_tpl.yaml` — same as `simple_tpl.yaml` but test with zero URLs to verify empty expansion behavior
- [x] 1.9 Create `empty_uses_tpl_output.yaml` — expected output: no expanded providers/groups, only passthrough entries + `clashtui: null`

## 2. Unit tests for template generation

- [x] 2.1 Write `test_simple_expansion` — load `simple_tpl.yaml`, run `gen_template()` with 2 URLs, assert output matches `simple_tpl_output.yaml`
- [x] 2.2 Write `test_multi_provider_expansion` — load `multi_provider_tpl.yaml`, run with 2 URLs, assert output matches `multi_provider_tpl_output.yaml`
- [x] 2.3 Write `test_no_tpl_param_passthrough` — load `no_tpl_param_tpl.yaml`, run with any URL list, assert output matches `no_tpl_param_tpl_output.yaml`
- [x] 2.4 Write `test_empty_uses` — load `empty_uses_tpl.yaml`, run with empty URL list, assert output matches `empty_uses_tpl_output.yaml`
- [x] 2.5 Write `test_ordering_preserved_proxy_groups` — verify that proxy-group order in output matches input order (Select → Auto(expanded) → Direct)
- [x] 2.6 Write `test_ordering_preserved_proxy_providers` — verify that proxy-provider order in output matches input (static providers before/after expanded ones)
- [x] 2.7 Write `test_angle_bracket_provider_placeholder` — verify `<pvd>` in `use` list expands to `[pvd0, pvd1]`
- [x] 2.8 Write `test_angle_bracket_group_placeholder` — verify `<Auto>` in `proxies` list expands to `[Auto-pvd0, Auto-pvd1]`
- [x] 2.9 Write `test_missing_proxy_providers_section` — assert `gen_template()` returns error when no `proxy-providers` key
- [x] 2.10 Write `test_missing_proxy_groups_section` — assert `gen_template()` returns error when no `proxy-groups` key
- [x] 2.11 Write `test_missing_tpl_param_providers_key` — assert error when `tpl_param` exists but no `providers` sub-key
- [x] 2.12 Write `test_placeholder_to_nonexistent_target` — assert error when `<>` references non-existent provider/group name or returns a graceful error
- [x] 2.13 Write `test_clashtui_marker_added` — verify generated output contains `clashtui: null`
- [x] 2.14 Write `test_path_generation_format` — verify expanded proxy-providers have `path` matching `proxy-providers/tpl/{tpl_name}/{pp_name}.yaml`

## 3. Robustness fixes to gen_template (if needed)

- [x] 3.1 Review `gen_template()` for potential panics on malformed YAML (unwrap on `as_str()`, `as_sequence()`, `as_mapping()`) and convert to proper `anyhow::Error` returns
- [x] 3.2 Ensure non-template proxy-provider entries that lack `url` or `path` are preserved as-is (not stripped)
- [x] 3.3 Verify that `<>` placeholder handling in `gen_template()` matches clashtui behavior: `use` entries with `<>` expand via `pp_names`, `proxies` entries with `<>` expand via `pg_names`
- [x] 3.4 Add explicit ordering comment blocks in `gen_template()` documenting why `Mapping` and `Sequence` guarantee output order

## 4. Documentation

- [x] 4.1 Create `docs/profile_template.md` with sections: Overview, Template YAML Format, Proxy-Provider Template Entries, Proxy-Group Template Entries, `<>` Placeholder Expansion, URL Sourcing (`clashtui.uses`), Full Workflow (create → configure → generate → select → deploy), Complete Example
- [x] 4.2 Include a full walkthrough example in the docs with real-looking YAML snippets
- [x] 4.3 Document the ordering guarantee explicitly: "Output YAML preserves the section ordering of the input template"
- [x] 4.4 Document the `ProfileType::Generated` database entry and how it affects subsequent profile operations (update, select, merge with basic config)

## 5. Verify

- [x] 5.1 Run `cargo check` to ensure no type errors
- [x] 5.2 Run `cargo test` to confirm all new tests pass
- [x] 5.3 Run `cargo test -- --nocapture` on test module to review test output for clarity
- [x] 5.4 Run `cargo build` for a full debug build
- [x] 5.5 Spot-check: manually create a template, run `apply_template()` via the TUI (or an inline test), and verify the generated profile is valid YAML
