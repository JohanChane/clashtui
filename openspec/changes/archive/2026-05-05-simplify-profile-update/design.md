## Context

After the `refactor-profile-storage` change, `ProfileType::Template` exists but stores empty `urls: []`. Template generation is a multi-step process: create Template profile → generate → update. The `profiles/` directory adds unnecessary complexity. The insight: template generation just produces a file — no different from importing a YAML file. So `ProfileType::Template` is redundant.

## Goals / Non-Goals

**Goals:**
- `ProfileType` has exactly two variants: `File` and `Url(String)`
- Template generation writes a YAML file and registers it as `ProfileType::File`
- `update_profile()` is uniform: read from `profile_yamls/`, merge, write to clash config
- `profiles/` directory and all associated code removed
- Legacy `!Template` / `!Generated` entries auto-migrate to `!File` on deserialization

**Non-Goals:**
- Changing `gen_template()` behavior
- Changing file import (`I` key) behavior
- Auto-migrating existing `profiles/` files

## Decisions

**Decision 1: Remove `ProfileType::Template` entirely**

`ProfileType` becomes:
```rust
pub enum ProfileType {
    File,
    Url(String),
}
```

Template generation is just a workflow: `gen_template()` → write to `profile_yamls/` → `ProfileType::File`. No special type needed.

**Decision 2: Legacy migration in serde**

`Wire` enum deserializes `!Template { template, urls }` and `!Generated(name)` as `Wire::File`. A log warning is emitted for each migration. `urls` data from `!Template` is discarded since File profiles don't carry URLs.

**Decision 3: `update_profile()` is the same for all types**

```rust
pub async fn update_profile(profile: Profile, with_proxy: bool, remove_proxy_provider: bool) -> Result<String> {
    // 1. Read file from profile_yamls/<name>.yaml
    // 2. Optionally remove_proxy_provider (update_profile_without_pp)
    // 3. Merge with basic config
    // 4. Write to clash config path
    // Done. No regeneration, no download.
}
```

File profiles are no longer "Not upgradable" — they update just like everything else.

**Decision 4: Template tab Generate creates File profile**

When user presses Enter in Template tab, `generate()` calls `apply_template()` which:
1. Runs `gen_template()` on the template YAML
2. Writes output to `profile_yamls/<name>.generated.yaml`
3. Registers as `ProfileType::File` (or updates existing File entry)

No URL collection, no Template record.

**Decision 5: Remove Profile tab's `ti` (add template)**

With no Template type, the `ti` keybinding is removed. Users generate from Template tab and the resulting File profile appears in the Profile tab automatically.

## Risks / Trade-offs

- **Legacy Template profiles lose their template reference** → Mitigation: File profiles don't need template references; re-generate from Template tab if needed
- **`!Template` `urls` data lost on migration** → Mitigation: `urls` was always empty in practice (template provides URLs directly)
