## 1. ProfileType schema change

- [x] 1.1 Add `ProfileType::Template { template: String, urls: Vec<String> }` variant to `src/config/database.rs` and `src/functions/file/profile/profile.rs`
- [x] 1.2 Add serde serialize/deserialize support for `ProfileType::Template` with YAML tags
- [x] 1.3 Add deserialization migration: `ProfileType::Generated(name)` → `ProfileType::Template { template: name, urls: [] }` with log warning
- [x] 1.4 Update `ProfileType` match arms across the codebase for the new variant (display, get_domain, etc.)

## 2. profile_yamls directory

- [x] 2.1 Add `PROFILE_YAMLS_DIR` constant and `profile_yamls_path()` function to `src/config.rs` (or `src/config/util.rs`)
- [x] 2.2 Add `profile_yamls/` directory creation to `init_config()` in `src/config.rs`
- [x] 2.3 Update file path constants in `src/functions/file.rs` (add PROFILE_YAMLS_PATH lazy static alongside PROFILE_PATH, TEMPLATE_PATH)

## 3. gen_template() refactor

- [x] 3.1 Change `gen_template()` signature from `(map, name, name_urls: Vec<(String, String)>)` to `(map, name, urls: Vec<String>)` in `src/functions/file/template/version1.rs`
- [x] 3.2 Remove `clashtui.uses` parsing logic from `gen_template()` — replace URL filtering with direct use of the `urls` parameter
- [x] 3.3 Remove `clashtui` key stripping logic from `gen_template()` (no longer present in template)

## 4. apply_template() update

- [x] 4.1 Update `apply_template()` in `src/functions/file/template.rs` to read `ProfileType::Template` from DB and pass `urls` to `gen_template()`
- [x] 4.2 Change output path from `profiles/<name>.clashtui_generated` to `profile_yamls/<name>.yaml`
- [x] 4.3 Update database registration to insert `ProfileType::Template` instead of `ProfileType::Generated`

## 5. Update test data and unit tests

- [x] 5.1 Remove `clashtui.uses` blocks from all test template YAML files in `src/functions/file/template/testdata/`
- [x] 5.2 Regenerate expected output YAML files if needed (remove any `clashtui` blocks, update paths)
- [x] 5.3 Update all 14 `gen_template()` unit tests in `version1.rs` to pass URLs directly instead of relying on template-embedded `clashtui.uses`
- [x] 5.4 Add tests for template profile with empty URL list (zero expansion)
- [x] 5.5 Add tests for `ProfileType::Template` serialization/deserialization in `database.rs` tests

## 6. File path import

- [x] 6.1 Add `import_profile_from_file(source_path, profile_name)` function in `src/functions/file/profile.rs` that copies file to `profile_yamls/` and registers as `ProfileType::File`
- [x] 6.2 Handle error cases: source not found, invalid YAML, target name conflict

## 7. TUI profile tab updates

- [x] 7.1 Add key binding and action for creating a Template-type profile (prompt for name, template file, and URL list)
- [x] 7.2 Add key binding and action for file path import (prompt for source path and profile name)
- [x] 7.3 Update profile listing/display to show template name and URL count for `ProfileType::Template` entries
- [x] 7.4 Update profile update action to handle `ProfileType::Template` (pass URLs from DB record)

## 8. Documentation update

- [x] 8.1 Update `docs/profile_template.md` to reflect removal of `clashtui.uses`, new profile-based URL sourcing, and `profile_yamls/` output
- [x] 8.2 Document file path import workflow in `docs/profile_template.md`

## 9. Verification

- [x] 9.1 Run `cargo check` — ensure no type errors
- [x] 9.2 Run `cargo test` — all template tests pass, all profile tests pass
- [x] 9.3 Run `cargo build` — clean release build
- [ ] 9.4 Manual smoke test: create template profile, generate, verify file appears in `profile_yamls/`
