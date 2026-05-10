## Why

demotui has a template-to-profile generation system (`src/functions/file/template/version1.rs`) that expands template YAML files into full clash configuration profiles. However, this system lacks tests, documentation, and explicit ordering guarantees — all of which clashtui's equivalent `crt_yaml_with_template()` already provides. Users and contributors need confidence that template expansion produces correct, deterministic output, especially when sections contain dozens of proxy-providers and proxy-groups.

## What Changes

- Harden the existing template generation function (`version1::gen_template`) with explicit order-preservation (already structurally correct via `serde_yml::Mapping`/`Sequence`, but needs verification and documentation)
- Add comprehensive unit tests covering: normal flow, empty subscription list, no `tpl_param` entries, `<>` placeholder expansion, section ordering, and error paths
- Write user-facing documentation at `docs/profile_template.md` explaining template format, `tpl_param` markers, `<>` substitution, and the full workflow from template creation to profile deployment
- Add test data (`src/functions/file/template/testdata/`) with sample template and expected output YAML files
- Ensure the `apply_template()` pipeline (`template.rs:47-76`) correctly wires URL sourcing (from `clashtui.uses` → database profile URLs) to `gen_template()` and writes the result to `profiles/`

## Capabilities

### New Capabilities
- `template-generation`: Expand a template YAML with `tpl_param` markers and `<>` placeholders into a fully resolved clash profile YAML, preserving input ordering in all sections
- `profile-serialization`: Serialize the generated profile to `profiles/<name>.yaml` with a `clashtui` marker, and register it in the profile database as `ProfileType::Generated`

### Modified Capabilities
<!-- None — this hardens existing behavior without changing requirements -->

## Impact

- Affected code: `src/functions/file/template/version1.rs` (may need minor robustness fixes), `src/functions/file/template.rs` (apply_template pipeline), new `src/functions/file/template/testdata/` directory
- New file: `docs/profile_template.md`
- No API changes, no dependency additions, no breaking changes
- The existing `Profile` / `ProfileType::Generated` / `ProfileManager` types remain unchanged
