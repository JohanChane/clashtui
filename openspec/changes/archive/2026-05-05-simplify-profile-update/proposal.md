## Why

The current profile update flow is overcomplicated with three profile types and regeneration logic. `ProfileType::Template` is unnecessary — template generation simply produces a clash YAML file, just like importing a file. All profiles should be either `File` or `Url`, and updating should just re-read the existing YAML file — no regeneration, no re-download.

## What Changes

- **Remove `ProfileType::Template`** — only `File` and `Url` remain. Template generation writes to `profile_yamls/` and registers as `ProfileType::File`.
- **`update_profile()` simplified** — all types read YAML from `profile_yamls/`, merge with basic config, write to clash config path. No regeneration, no download.
- **`profiles/` directory removed** — all profile YAML lives in `profile_yamls/`. `PROFILE_PATH` constant/lazy static deleted. `update_with()` download logic removed.
- **BREAKING**: `!Template` entries in database are auto-migrated to `!File` on load. Old `profiles/` files are not migrated.
- **BREAKING**: Template tab `add_template` (`ti`) becomes the same as Generate — produces a File-type profile. No separate Template profile concept.

## Capabilities

### New Capabilities

None — this is pure removal/simplification.

### Modified Capabilities

- `profile-yamls-storage`: `profile_yamls/` becomes the sole directory for all profile YAML. `profiles/` directory removed. `load_local_profile()` reads all types from `profile_yamls/`.
- `profile-serialization`: `ProfileType` reduced to `File | Url(String)`. Legacy `!Template` and `!Generated` deserialize as `!File`.
- `template-generation`: `gen_template()` unchanged. `apply_template()` writes file to `profile_yamls/` and registers as `ProfileType::File`.

### Removed Capabilities

- `template-profile-record`: `ProfileType::Template` variant is deleted.
- `template-url-extraction`: No longer needed — no Template record to store URLs in.
- `file-path-import`: Template generation replaces explicit file import for template profiles (user can still import arbitrary files with `I`).

## Impact

- `src/config/database.rs` — remove `ProfileType::Template` variant and its custom serde; remove `Wire::Template`, `Wire::Generated`; simplify serde
- `src/config.rs` / `src/config/util.rs` — remove `PROFILE_DIR`, `profile_path()`
- `src/functions/file.rs` — remove `PROFILE_PATH` lazy static
- `src/functions/file/profile.rs` — remove `update_with()`, simplify `update_profile()`, simplify `db::create()`, remove Template match arms
- `src/functions/file/profile/profile.rs` — `load_local_profile()` always reads from `PROFILE_YAMLS_PATH`, `get_domain()` simplified
- `src/functions/file/template.rs` — `apply_template()` registers as `ProfileType::File`
- `src/tui/tab/files/profile.rs` — remove `add_template`, `ImportFile` unchanged; Template tab `ti` keybinding removed
- `src/functions/file/template/version1.rs` — `gen_template()` unchanged
