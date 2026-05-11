## Context

Currently, proxy-provider URLs for template profiles are stored in two places: a standalone `template_proxy_providers.yaml` file (keyed by group name) and a cloned copy inside `clashtui.db` under `ProfileType::Template { proxy_provider_groups }`. This dual-storage creates consistency issues — the two can diverge after edits. The design doc (commit `502cc5b8f271`) resolves this by making each template file the single source of truth: URLs live under a `clashtui.proxy_provider_groups` key inside the template YAML, and the database stores only the template filename.

The `template-profile-record` spec currently describes `ProfileType::Template` with a `urls: Vec<String>` field (flat list). The actual code uses `proxy_provider_groups: ProxyProviderGroups` (nested `HashMap<String, BTreeMap<String, String>>`). Both conflict with the new design.

## Goals / Non-Goals

**Goals:**
- Eliminate `proxy_provider_groups` field from `ProfileType::Template` in `clashtui.db`
- Store proxy-provider URLs exclusively in template files under `clashtui.proxy_provider_groups`
- Generated profile output includes `clashtui.proxy_provider_groups` for self-description
- Template generation and update flows read URLs from the template file, not from the database or a standalone file
- Backward-compatible deserialization of old `clashtui.db` entries with embedded groups
- Unified approach for both Mihomo and sing-box

**Non-Goals:**
- Changing the proxy-provider expansion logic (`tpl_param`, `${PPG}`, `${PGG}` placeholders) — that logic stays the same
- Changing the profile selection merge (with `core_override_config`) — that stays the same
- Adding a UI for editing `clashtui.proxy_provider_groups` inline — editing uses the existing `edit_cmd` to open the template file

## Decisions

### D1: URLs live in template files under `clashtui.proxy_provider_groups`

**Decision**: The `clashtui` key in template YAML files holds `proxy_provider_groups`, a mapping from group name to `{provider_name: url}`.

```yaml
clashtui:
  proxy_provider_groups:
    pvd:
      pvd0: https://example.com/sub1.yaml
      pvd1: https://example.com/sub2.yaml

# rest of template content...
```

**Rationale**: Each template file is self-contained. Users edit the template file directly (via `edit_cmd`) to manage URLs. No separate file to synchronize. The generated profile output also carries this metadata, so users can see which URLs went into a given profile.

**Alternatives considered**:
- Keep `template_proxy_providers.yaml` as a global file shared by all templates. Rejected: doesn't allow per-template URL sets; still a separate file to sync.
- Store URLs in `clashtui.db` only. Rejected: database is opaque; users can't easily inspect or edit URLs; still a duplication risk.

### D2: Database stores only template filename

**Decision**: `ProfileType::Template` becomes `Template { template: String }`.

```rust
enum ProfileType {
    File,
    Url(String),
    Template { template: String },
    Singbox,
}
```

`clashtui.db` YAML format:
```yaml
common_tpl.yaml.tpl:
  dtype: !Template
    template: common_tpl.yaml
  no_pp: false
```

**Rationale**: No URL duplication. The database only needs to know which template file this profile derives from. URLs are always fetched from the template file on demand.

### D3: Legacy deserialization with migration

**Decision**: On deserializing an old `!Template { template, proxy_provider_groups }` entry:
1. Log a warning: `"Migrating legacy Template profile '{name}': writing proxy_provider_groups to template file"`
2. Write `clashtui.proxy_provider_groups` into the template YAML file (if the groups are non-empty and the file doesn't already have a `clashtui.proxy_provider_groups` key)
3. Deserialize as `ProfileType::Template { template }` (discard the groups in-memory)

Similarly, `!Generated "name"` is still migrated to `Template { template: "name" }`.

**Rationale**: Users who upgrade get automatic migration. The old data isn't lost — it's moved to the canonical location. This is a one-way migration (no rollback).

### D4: Generated profile output includes `clashtui.proxy_provider_groups`

**Decision**: `gen_template()` and `gen_template_singbox()` inject a top-level `clashtui` key with `proxy_provider_groups` before writing the output to `profiles/<name>.yaml`.

This replaces the previous `clashtui: null` marker (spec'd in `template-generation` spec).

```yaml
clashtui:
  proxy_provider_groups:
    pvd:
      pvd0: https://example.com/sub1.yaml
      pvd1: https://example.com/sub2.yaml

# expanded template content...
```

**Rationale**: Generated profiles become self-describing. When updating a template profile, the system can read URLs either from the template file (for generation) or from the generated profile (for verification). The `clashtui` key is harmless — mihomo ignores unknown top-level keys.

### D5: Template profile update only downloads, does not regenerate

**Decision**: `update_template_profile()` only downloads proxy-provider files to the cache directory. It does NOT call `apply_template()` to regenerate the profile. Profile regeneration only happens when the user explicitly presses Enter on a template in the TUI.

During update, the proxy-provider URLs are read from the **generated profile file** (`profiles/<name>.yaml`) via its `clashtui.proxy_provider_groups` key. During generation (Enter on template), URLs are read from the **template file** (`templates/<name>`) via its `clashtui.proxy_provider_groups` key.

**Rationale**: The generated profile in `profiles/` already has proxy-providers expanded with `path:` fields pointing to the cache directory. When proxy-provider files are updated (re-downloaded), those `path:` references remain valid — the core reads the updated files from the same paths. Regeneration is unnecessary and wastes time fetching subscriptions again during update.

After a successful update, if the updated profile is the currently active profile, a re-select is triggered automatically so the core reloads with the newly downloaded proxy-provider files.

### D6: Template profile selection requires proxy-provider file availability

**Decision**: Before using a template-generated profile as the core config, verify that all proxy-provider files (downloaded to the proxy-providers cache directory) exist. If any is missing, refuse to select and show an error.

**Rationale**: Template profiles expand `${PPG.x}` placeholders into `use: [pvd0, pvd1]` references. If the proxy-provider files haven't been downloaded, the core would fail to load. Checking file existence prevents this.

### D7: `read_template_proxy_providers()` becomes `read_template_ppg(template_name)`

**Decision**: Replace the global `read_template_proxy_providers()` (which reads the standalone YAML file) with a function that reads `clashtui.proxy_provider_groups` from a specific template file. The old function is removed.

```rust
pub fn read_template_ppg(template_name: &str) -> anyhow::Result<ProxyProviderGroups>;
```

**Rationale**: Each template file is now self-contained. The function needs to know which template to read.

## Risks / Trade-offs

- **One-way migration**: Old databases are migrated on first load. If a user needs to downgrade, they lose the URL-group mapping in the database. → Migration is logged prominently; users on stable releases rarely downgrade.
- **Template file editing**: Users must now edit the template file to change URLs (there's no dedicated "edit proxy provider URLs" action that opens a separate file). The `_edit_providers` key action now opens the template file itself. → This is simpler for the user (one file to edit).
- **`clashtui` key in generated profile**: If mihomo/sing-box ever reserves `clashtui` as a config key, it could conflict. → Extremely unlikely; `clashtui` is namespace-prefixed.

## Migration Plan

1. On startup, `ProfileManager` deserializes `clashtui.db` with the new backward-compatible deserialization.
2. For each old `!Template { template, proxy_provider_groups }` entry encountered:
   - Read the template file at `templates/<template>`
   - If it doesn't already have `clashtui.proxy_provider_groups`, write the migrated groups into it
   - Log the migration
3. On save, the database is written in the new format (without groups).
4. The standalone `template_proxy_providers.yaml` file is no longer read or written, but is not auto-deleted (user can remove it manually).

## Open Questions

- ~~Should `template_proxy_providers.yaml` be auto-deleted on migration?~~ No — leave it for manual cleanup.
- ~~Does sing-box template generation need the same `clashtui.proxy_provider_groups` injection?~~ Yes, for consistency and self-describing profiles.
