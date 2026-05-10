## Why

In clashtui, "no proxy-provider" (no_pp) is a global toggle. Flipping it affects whatever profile happens to be selected, requiring manual switching between profiles with different needs. Each profile has its own ideal config — some should embed proxy-providers, others should keep them external. Moving no_pp to per-profile storage in `.db` makes the preference persistent per-profile and eliminates manual toggling.

## What Changes

- **Per-profile `no_pp` in database** — `ProfileType` is replaced with a `ProfileData` struct stored in `ProfileManager` that wraps `dtype: ProfileType` and `no_pp: bool`. The database YAML format changes from flat enum values to structured fields.
- **TUI toggle key** — a new keybinding on the Profile tab toggles `no_pp` for the selected profile and persists to `.db` immediately.
- **Update flow uses stored preference** — `update_profile()` reads `no_pp` from the profile database entry instead of using a hardcoded `false`.
- **Backward-compatible deserialization** — old database format (`pf1: File`, `pf2: !Url "...`) deserializes to `ProfileData { dtype, no_pp: false }`. New format serializes with the flag.

## Capabilities

### New Capabilities
- `profile-serialization`: Per-profile metadata storage (`no_pp` flag) in the database with backward-compatible YAML serialization.

### Modified Capabilities
- `profile-serialization`: Database schema changes from `HashMap<String, ProfileType>` to `HashMap<String, ProfileData>` with backward-compatible deserialization.

## Impact

- `src/config/database.rs` — replace `ProfileType` in `ProfileDataBase` with new `ProfileData` struct; update `ProfileManager` methods; backward-compat serde
- `src/config.rs` — `Config::data` type changes from `Mutex<ProfileManager>` (no signature change, internal only)
- `src/functions/file/profile.rs` — `db::create()` accepts `no_pp` flag; `update_profile()` uses `profile.no_pp` instead of parameter
- `src/functions/file/template.rs` — `apply_template()` uses `ProfileType::File` with `no_pp: false`
- `src/tui/tab/files/profile.rs` — add toggle keybinding; `update()`/`update_all()` read `no_pp` from profile
