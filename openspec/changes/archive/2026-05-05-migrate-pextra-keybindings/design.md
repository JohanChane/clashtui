## Context

The `ConfigFile` struct already has an `open_dir_cmd` field with platform-appropriate defaults (`open %s` / `start %s`). The `edit()` function in `functions/command.rs` already uses the identical pattern for `edit_cmd`. What's missing is a function that reads `open_dir_cmd` and two key bindings that invoke it to open the clash config directory and the demotui config directory.

The config directory is stored in the private `DATA_DIR: OnceLock<PathBuf>` with individual path accessors (`template_path()`, `profile_yamls_path()`, etc.) but no direct config-dir accessor.

## Goals / Non-Goals

**Goals:**
- Add `open_dir(path: &str)` to `functions/command.rs` mirroring the existing `edit()`
- Add `Ctrl+g c` chord: open the demotui config directory
- Add `Ctrl+g m` chord: open the clash config directory
- Add `e` key to Template tab for editing template files (mirrors the existing Profile tab edit)
- Update the help panel's global shortcuts list

**Non-Goals:**
- CLI flags for `edit_cmd` / `open_dir_cmd`
- Config migration (already handled by `migration_v0_2_3`)
- Keymap.yaml overrides for global chords (global chords are hardcoded, not tab-level)

## Decisions

### 1. `open_dir()` mirrors `edit()` identically

Follow the existing one-liner pattern in `functions/command.rs:77-81`:

```rust
pub fn open_dir(path: &str) -> Result<()> {
    spawn("sh", vec!["-c", CONFIG.cfg_file.open_dir_cmd.replace("%s", path).as_str()])
}
```

No fallback to `xdg-open` is needed — the config default already provides `open %s` / `start %s`.

### 2. Expose config directory path in `config.rs`

Add `pub fn config_dir_path() -> PathBuf` returning `DATA_DIR.get().unwrap().clone()`. This is consistent with the existing pattern of `template_path()`, `profile_yamls_path()`, etc.

### 3. `Ctrl+g c` / `Ctrl+g m` implemented as a global chord layer

Use a separate `ChordHandler` (`global_chord` field on `App`) at a new layer inserted between PopUp (layer 0) and Help (layer 1). This layer checks whether the key sequence matches `Ctrl+g` followed by `c` or `m`.

**Why a separate ChordHandler instead of integrating into the tab-level chord handler?**

The tab-level chord handler dispatches to tab-specific actions via `dispatch_shortcut()`. Global chords need a different dispatch — they call `open_dir()` synchronously. A separate handler avoids coupling global and tab-level concerns.

**Why between PopUp and Help?**

PopUp takes highest priority (user dialogs must work). If a PopUp is active, the global chord is blocked. If the global chord prefix (`Ctrl+g`) is pressed, it enters chord-pending state and Help is not dismissed — the user is mid-sequence. If a non-matching key is pressed during a pending global chord, the chord is cancelled and the key falls through to the remaining layers normally.

**Layer order after this change:**

```
Layer 0:   PopUp         — if active, return
Layer 0.5: Global Chord  — if handled (prefix or complete), return (NEW)
Layer 1:   Help          — if active, dismiss and return
Layer 2:   Tab Chord     — if handled, return
Layer 3:   Tab handler   — always called (no short-circuit)
Layer 4:   Global keys   — always called
```

### 4. `Ctrl+g` prefix avoids all conflicts

`Ctrl+g` is not used anywhere in the current codebase (verified across all tab `mod_agent!` definitions and global `handle_global_kv`). The `c` and `m` second keys are lowercase and plain (no modifiers), avoiding confusion with `Ctrl+c` (quit) and `Ctrl+m` (Enter-equivalent).

### 5. Template edit follows the exact Profile edit pattern

The Template tab gets `e` → `Edit` action in `mod_agent!`, an `Edit` variant on the `Action` enum, an `Action::Edit` arm in `act()`, and an `async fn _edit(name: String) -> CB` callback. The template path is constructed as `config::template_path().join(&name)` — no database lookup needed since templates are just files in the templates directory. See `src/tui/tab/files/profile.rs:298-303` for the reference implementation.

### 6. Help panel documents the new chords

The help panel's global shortcuts section uses the `KeyCombo` array format. Add `Ctrl+g c` and `Ctrl+g m` entries using the `key_event_to_str()` helper (already used for rendering chord sequences in the tab section).

## Risks / Trade-offs

- **Clash config dir may not exist or be empty** → `open_dir()` spawns the command regardless; the external file manager handles the "directory not found" UX
- **`Ctrl+g` may be intercepted by some terminals** → On most terminal emulators (kitty, alacritty, wezterm), `Ctrl+g` is not intercepted and reaches the application as an input event. The original clashtui used plain `G`/`H` without issues.
