## 1. Database model changes

- [x] 1.1 Add `ProfileData` struct with `dtype: ProfileType` and `no_pp: bool`, plus `ProfileDataWire` enum for backward-compat serde
- [x] 1.2 Implement `Serialize`/`Deserialize` for `ProfileData` via `ProfileDataWire`
- [x] 1.3 Change `ProfileDataBase` from `HashMap<String, ProfileType>` to `HashMap<String, ProfileData>`
- [x] 1.4 Add `no_pp: bool` field to `Profile` struct, update `Default`
- [x] 1.5 Update `ProfileManager::insert()` to wrap `ProfileType` in `ProfileData { dtype, no_pp: false }`
- [x] 1.6 Update `ProfileManager::get()` and `remove()` to populate `Profile.no_pp` from `ProfileData`
- [x] 1.7 Add `ProfileManager::set_no_pp(name, value)` to toggle and persist the flag
- [x] 1.8 Update database tests for new format (roundtrip, backward compat, migration)

## 2. update_profile signature change

- [x] 2.1 Remove `remove_proxy_provider: bool` parameter from `update_profile()`, use `profile.no_pp` instead
- [x] 2.2 Update `db::create()` callers (no signature change needed — `insert` defaults to `false`)
- [x] 2.3 Verify `import_profile_from_file()` works (no changes needed — `insert` defaults to `false`)
- [x] 2.4 Verify `apply_template()` works (no changes needed)

## 3. TUI toggle keybinding

- [x] 3.1 Add `Action::ToggleNoPp` variant to the `Action` enum in profile tab
- [x] 3.2 Add `KeyCode::Char('N')` → `Action::ToggleNoPp` to the `mod_agent!` block
- [x] 3.3 Implement `ToggleNoPp` handler: read current `no_pp`, flip it, save database, refresh list

## 4. TUI update callers

- [x] 4.1 Remove `let remove_proxy_provider = false;` in `update()` and update `update_profile()` call
- [x] 4.2 Remove hardcoded `false` from `update_all()` and update `update_profile()` call
- [x] 4.3 Add `atime` display for `no_pp` status (show `|nopp` in profile list with pipe separator)

## 5. Tests and verification

- [x] 5.1 Add test: old database format deserializes with `no_pp: false`
- [x] 5.2 Add test: new database format roundtrip preserves `no_pp`
- [x] 5.3 Add test: `set_no_pp` toggles value and persists
- [x] 5.4 Run full test suite, verify 50+ pass (same 2 pre-existing chord failures)
- [x] 5.5 Run `cargo check` clean
