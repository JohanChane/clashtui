## Context

In clashtui, `no_pp` (no proxy-provider) is a global boolean stored in `ClashtuiData`. Toggling it affects the CURRENTLY selected profile when `select_profile()` or `update_profile()` runs. The TUI provides a global `switch_no_pp()` that flips the state for all profiles.

Currently in demotui, `remove_proxy_provider` is hardcoded `false` in the TUI update functions — there is no toggle at all.

The `ProfileManager` stores `HashMap<String, ProfileType>` in YAML (`profiles.yaml` — the `.db` file). `ProfileType` is an enum with `File` and `Url(String)` variants. There is no per-profile metadata beyond the type.

## Goals / Non-Goals

**Goals:**
- Store `no_pp: bool` per profile in the database (`.db` / `profiles.yaml`)
- Provide a TUI keybinding to toggle `no_pp` for the selected profile
- `update_profile()` reads `no_pp` from the profile struct instead of a parameter
- Backward-compatible deserialization: old database files load with `no_pp: false`

**Non-Goals:**
- Changing `no_pp` toggle visibility in the profile list UI (just keybinding for now)
- Adding `no_pp` to the `add`/`import` input flows (defaults to `false`)
- Renaming `update_profile_without_pp` function (name still conveys the concept)

## Decisions

### 1. New `ProfileData` struct wrapping `ProfileType` + `no_pp`

Replace `HashMap<String, ProfileType>` with `HashMap<String, ProfileData>`:

```rust
#[derive(Clone, Debug, Default)]
pub struct ProfileData {
    pub dtype: ProfileType,
    pub no_pp: bool,
}
```

**Rationale**: This keeps the `Profile` struct (runtime representation) compatible while adding the flag to each database entry. No need for a separate flag map.

**Alternative considered**: Adding a separate `HashMap<String, bool>` alongside the profiles map. Rejected — more code, harder to keep in sync.

### 2. Backward-compatible serde via `#[serde(untagged)]` wire enum

```rust
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum ProfileDataWire {
    New {
        dtype: ProfileType,
        #[serde(default)]
        no_pp: bool,
    },
    Old(ProfileType),
}
```

`ProfileData` implements `Serialize`/`Deserialize` by converting to/from `ProfileDataWire`. The `#[serde(untagged)]` tries `New` (mapping) first, then falls back to `Old` (bare value).

Old format: `pf1: File` → `ProfileData { dtype: File, no_pp: false }`
New format: `pf1: { dtype: File, no_pp: true }`

### 3. `update_profile()` drops the `remove_proxy_provider` parameter

```rust
pub async fn update_profile(profile: Profile, with_proxy: bool) -> anyhow::Result<String>
```

The preference is read from `profile.no_pp`. Callers (`update()`, `update_all()`) no longer pass a hardcoded `false`.

### 4. Toggle keybinding: `N` on the Profile tab

Adds `Key::Action(Action::ToggleNoPp)` bound to `KeyCode::Char('N')`. The handler toggles `no_pp` in the database and refreshes the list. No modal or confirmation needed — immediate toggle with visual feedback via the profile list.

### 5. `db::create()` defaults `no_pp` to `false`

New profiles start with `no_pp: false`. The user toggles it later. Same for `import_profile_from_file()` and `apply_template()`.

## Risks / Trade-offs

- **Database format change** → Existing `profiles.yaml` files are automatically migrated on load (old entries get `no_pp: false`). The file is rewritten with the new format on next save.
- **`Profile` struct size increase** → Negligible — one `bool` per profile.
- **Toggle without visual indicator** → User must know `N` toggles it. A future change could add a column/info display.
