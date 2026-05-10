## Context

The profile and template tabs use `DualTab<Profile, Template>` with `ListState` for cursor tracking. Both lack several convenience features present in the Connections tab (`dd` delete with confirmation, `G` for jump-to-end, `j`/`k` movement, `/` search in template). Cursor positions in `ListState` are preserved implicitly across tab switches (the state struct lives on the heap and is never dropped), but there's no validation that the selected index is still valid after async data reloads.

## Goals / Non-Goals

**Goals:**
- Add `dd` chord with Confirm popup before file deletion in profile and template
- Add `j`/`k` movement keys to profile and template
- Replace `ge` chord with single `G` for jump-to-end in profile and template
- Add `gg`, `G` to template tab (template already has `gg`)
- Add `/` search PopUp to template tab
- Ensure cursor defaults to first item on initial data load and stays valid after data reloads
- Re-export profile import (`I`) key binding to the DualTab shortcuts table

**Non-Goals:**
- Changing keybindings in Settings, Status, SrvCtl tabs
- Adding `/` search to Proxies tab
- Changing the DualTab focus-switch mechanism
- Modifying the chord system (`ChordHandler`) internals
- Changing the yakonfig/keymap.yaml format

## Decisions

### 1. `dd` chord replaces single `d` for delete

**Decision**: Remove single-key `d` from profile and template `mod_agent!`, add `dd` chord. Implement delete with Confirm popup, mirroring Connections' `Terminate` pattern.

**Rationale**: The chord system gives single-key entries priority over chord prefixes (`chord.rs:78-82`). A single `d` binding makes `dd` unreachable. Connections uses `dd` for terminate and shows a confirmation popup — profile/template should follow this pattern for consistency.

**Alternative considered**: Keep `d` as single-key delete (no confirmation), add `dd` as a separate "delete all" chord. Rejected — accidental single-key deletion of profile/template files is too destructive without confirmation.

### 2. `ge` chord replaced by `G`

**Decision**: Replace `([g, e], GoEnd)` with `([G], GoEnd)` in profile and template keymaps. `G` produces `KeyCode::Char('G')` with `shift: true` from crossterm, which maps naturally.

**Rationale**: Matches Connections tab convention (`G` for GoBottom). Vim-style muscle memory expects `G` for end-of-list, not `ge`. The `G` key is already used in Connections without issue.

### 3. Cursor position memory with validity clamping

**Decision**: On initial data load (`init`), set `ListState::selected` to `Some(0)`. On every data reload (`sync_helper` or equivalent), clamp the selected index to `< items.len()`. For Connections, add clamping after `refresh_display_rows()`.

**Where to clamp**:
- **Profile**: In `sync_helper()` (already the consolidation point for item list changes)
- **Template**: In the `init` async block and potentially in `sync_header` or equivalent (template has no sync_helper — add a clamp in the init handler after setting items, and add a `sync_header` or clamp inline in any future content-changing handler)
- **Connections**: Add clamp in `refresh_display_rows()` 
- **Proxies**: Add clamp after `rebuild_from_proxies()` calls in both `init` and `after_sync`

**Rationale**: `ListState` already preserves position across tab switches (it's never dropped). The problem is stale indices after content changes. Clamping at content-update points ensures the cursor is always valid without requiring per-action adjustment.

### 4. Template delete implementation

**Decision**: Implement template file deletion by removing the file from `TEMPLATE_PATH/{name}` and also removing any associated profile records. Use the existing template path resolution.

**Rationale**: Template files are standalone YAML files in the config directory's `templates/` folder. Deleting them is a simple file removal. Since template-generated profiles reference templates, deleting a template should warn but not cascade-delete profiles (users can choose to remove profiles separately).

### 5. Template search implementation

**Decision**: Reuse the profile search pattern: add `Action::Search` variant, `/` key binding, and an async `search()` function that opens an Input PopUp with "Filter" title and saves to `self.filter`.

**Rationale**: Template already has `filter: Option<String>` and rendering logic for filtered display. Only the keybinding and PopUp trigger are missing. Copying the profile pattern ensures consistency.

### 6. Profile import action

**Decision**: The `I` (ImportFile) binding already exists in profile's `mod_agent!` and the `import_file()` async handler is implemented. No code changes needed for this feature — it's already present. The proposal documents its existence.

**Rationale**: The import from local file functionality (`I` key) is fully implemented (profile.rs:274-296). The user may have been unaware it exists due to lack of discoverability in the TUI — the title bar shows "i: Add" but not "I: Import file".

## Risks / Trade-offs

- **[Breaking] `ge` → `G`**: Users with muscle memory for `ge` need to adapt. Users with custom keymap.yaml overrides using `ge` will see those overrides stop working until updated.
- **[Breaking] `d` → `dd`**: Single `d` for delete no longer works. Users with keymap.yaml overrides mapping single `d` will need to update to `dd` chord.
- **Template deletion scope**: Currently templates have an associated `ProfileType::Template` record with URLs. Deleting a template file will orphan those profile records if not cleaned up. Decision is to warn but not cascade — this could leave dangling references.
- **Cursor clamping with ListState**: `ListState` is ratatui's built-in. The `options.offset` field is independent of `options.selected` and is not clamped automatically by ratatui. We only clamp `selected`, not `offset`, which could cause visual offset issues on very long lists. Mitigation: ratatui's `List` widget handles offset naturally; no additional action needed.

## Open Questions

- Should template deletion also remove associated `ProfileType::Template` database records? Recommendation: yes, but needs user research on expected behavior.
- Should `G` also be added to Proxies tab for consistency? Out of scope for this change, but worth considering.
