## 1. Remove ProfileType::Template variant

- [x] 1.1 Simplify `ProfileType` enum to `File | Url(String)` in `src/config/database.rs`
- [x] 1.2 Simplify custom `Serialize` for `ProfileType` — remove `Template` variant, keep `File` and `Url`
- [x] 1.3 Simplify `Wire` enum for deserialization — `File | Url(String)`; map `Template` and `Generated` to `Wire::File` with log warning
- [x] 1.4 Remove all match arms on `ProfileType::Template { .. }` across codebase
- [x] 1.5 Update `get_domain()` — remove Template arm

## 2. Remove profiles/ directory usage

- [x] 2.1 Delete `PROFILE_DIR` constant from `src/config/util.rs`
- [x] 2.2 Remove `profile_path()` function from `src/config.rs`
- [x] 2.3 Remove `PROFILE_DIR` directory creation from `init_config()` in `src/config.rs`
- [x] 2.4 Delete `PROFILE_PATH` lazy static from `src/functions/file.rs`
- [x] 2.5 Remove all imports of `PROFILE_PATH` across the codebase

## 3. Simplify load_local_profile

- [x] 3.1 Change `load_local_profile()` to always use `PROFILE_YAMLS_PATH.join(format!("{name}.yaml"))` for all types

## 4. Simplify update_profile

- [x] 4.1 Remove `update_with()` function from `src/functions/file/profile.rs`
- [x] 4.2 Replace `update_profile()` body: read file from `profile_yamls/`, optionally call `update_profile_without_pp()`, merge with basic config, write to clash config path
- [x] 4.3 Remove all type-specific match arms from update path

## 5. Simplify apply_template

- [x] 5.1 Change `apply_template()` to register generated profile as `ProfileType::File` (not `Template`)
- [x] 5.2 Remove `urls` field and any URL extraction logic
- [x] 5.3 `update_profile_without_pp()` — no Template-specific logic; unchanged

## 6. Update TUI call sites

- [x] 6.1 Remove `add_template` action and `ti` keybinding from Profile tab (`src/tui/tab/files/profile.rs`)
- [x] 6.2 Template tab Generate (Enter) → calls `apply_template()` which creates File profile; works with new signature
- [x] 6.3 File import (`I` key) unchanged
- [x] 6.4 Remove Template-type display text from `get_profiles_with_readable_atime()`

## 7. Update Profile creation for Url type

- [x] 7.1 Change `add()` action to download to `profile_yamls/<name>.yaml` first, then call `db::create()`
- [x] 7.2 Ensure `profile_yamls/` exists before writing (via `create_dir_all`)

## 8. Clean up dead code and imports

- [x] 8.1 Remove unused imports (`PROFILE_PATH`, `File` import, `apply_template` from profile.rs, `AddTemplate` variant)
- [x] 8.2 Remove dead `database.rs` in `src/functions/file/profile/` (unused, referenced old `ProfileType::Generated`)

## 9. Update tests

- [x] 9.1 Update `serde_template` test → `serde_template_migrated_to_file` verifies `!Template` deserializes as `!File`
- [x] 9.2 `serde_template_empty_urls` test merged into `serde_template_migrated_to_file`
- [x] 9.3 `serde_migration_generated_to_template` → `serde_generated_migrated_to_file` verifies `!Generated` deserializes as `!File`
- [x] 9.4 Run `cargo test` — 48 passed, 2 pre-existing failures (chord handler)
- [x] 9.5 Run `cargo check` — no errors

## 10. Update documentation

- [x] 10.1 Update `docs/profile_template.md` — remove `profiles/` directory, remove Template profile type, remove `ti` keybinding