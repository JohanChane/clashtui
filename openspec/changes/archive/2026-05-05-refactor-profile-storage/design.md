## Context

The current template-to-profile system (`src/functions/file/template/`) embeds URL sourcing in the template YAML via `clashtui.uses`. This tightly couples templates to specific profiles, making templates less reusable. The URL list should live at the profile level so different profiles can use the same template with different subscription URLs.

Additionally, generated profiles are currently written to `profiles/` — the same directory as source/URL profiles — with no dedicated directory for clash YAML configurations. A separate `profile_yamls/` directory is needed for generated outputs and imported file copies.

The existing `ProfileType::Generated(String)` stores only a template name, not URLs. The refactor changes this to a richer record.

## Goals / Non-Goals

**Goals:**
- Remove `clashtui.uses` from template YAML — templates become pure clash config with only `tpl_param` markers
- Introduce a new profile database record type (`ProfileType::Template`) that stores template name + URL list
- Move generated profile output from `profiles/` to a new `profile_yamls/` directory
- Add file path import: copy a local YAML to `profile_yamls/` and register as a profile
- Update `gen_template()` to accept URLs from the profile record instead of reading `clashtui.uses`
- Update the TUI profile tab to create/edit template profiles with URL lists

**Non-Goals:**
- Changing template expansion logic (`tpl_param`, `<>` substitution, ordering)
- Adding new template format versions
- Token/authentication handling in URL downloads
- Changing clash core integration or service control
- Moving or renaming the `profiles/` directory itself (only generated output relocates)

## Decisions

### 1. New `ProfileType::Template` variant replaces `ProfileType::Generated`

`Generated(String)` stored only the template name. The new variant stores both the template name and the URL list:

```rust
pub enum ProfileType {
    File,
    Url(String),
    Template { template: String, urls: Vec<String> },
}
```

**Rationale**: The profile record is the natural place for "which template + which URLs". This is what the TUI user configures — pick a template, list the subscription profile names (or raw URLs) to feed into it. The `Generated` variant was added in the still-active `template-profile-generation` change and has no deployed consumers, so the rename is safe.

**Alternative considered**: Keep `Generated(String)` and add URLs to a separate file. Rejected — two places for one concern is error-prone.

### 2. Dedicated `profile_yamls/` directory

A new directory constant `PROFILE_YAMLS_DIR = "profile_yamls"` under `DATA_DIR`. This stores:
- Generated YAML from template expansion: `profile_yamls/<profile_name>.yaml`
- Copies of imported files: `profile_yamls/<profile_name>.yaml`

The existing `profiles/` directory remains for source/URL-downloaded profiles (the raw subscription YAML).

**Rationale**: Separating "source" profiles (downloaded from URLs, user-edited) from "consumable" clash YAML (generated, imported copies) avoids ambiguity about which file clash should load.

### 3. `gen_template()` receives URLs directly from caller

Current signature: `gen_template(map: &mut Mapping, name: &str, name_urls: Vec<(String, String)>)` — it reads `clashtui.uses` from the template, then filters `name_urls` by profile name.

New signature: `gen_template(map: &mut Mapping, name: &str, urls: Vec<String>)` — the caller passes the URL list directly from the profile record. The `clashtui.uses` block is no longer present in the template YAML (the caller strips it before calling `gen_template()`, or it simply doesn't exist).

**Rationale**: Simplifies the function. The profile record (caller) owns the URL list; `gen_template()` just expands. No template-to-profile-name filtering needed.

### 4. File import copies to `profile_yamls/`

Importing a local YAML file:
1. Copy the source file to `profile_yamls/<name>.yaml`
2. Register in profile database as `ProfileType::File`
3. The copied file IS the clash config — no further processing

**Rationale**: `profile_yamls/` is the canonical location for clash-ready YAML. Copying ensures the file lives under managed storage and won't disappear if the source moves.

### 5. Output filename convention

Generated profiles use `profile_yamls/<name>.yaml` (plain `.yaml` extension, no `.clashtui_generated` suffix). The suffix is unnecessary since `profile_yamls/` only contains clash-ready YAML.

## Risks / Trade-offs

- **Breaking change for existing template YAML**: Templates with `clashtui.uses` will fail after the change. → Mitigation: The template-profile-generation change introducing `clashtui.uses` has not been deployed/released yet. No users are affected.
- **`ProfileType::Generated` removal**: Any code serializing/deserializing `Generated` needs updating. → Mitigation: Add a migration in the YAML deserializer to auto-convert old `Generated` entries to `Template` with an empty URL list (and warn).
- **`profile_yamls/` directory must exist**: The config init code (`init_config()`) must create it. → Mitigation: Add directory creation in `init_config()` alongside `profiles/` and `templates/`.
