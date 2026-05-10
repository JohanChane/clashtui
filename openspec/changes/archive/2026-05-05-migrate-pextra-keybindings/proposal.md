## Why

The `open_dir_cmd` config field exists in `ConfigFile` and is correctly loaded from YAML, but it is dead code — no function reads it and no TUI key binding invokes it. Users migrating from clashtui expect `G` (open clash config directory) and `H` (open clashtui config directory) global shortcuts to work, as documented in the original project's usage guide.

## What Changes

- Add `open_dir()` public function to `src/functions/command.rs`, mirroring the existing `edit()` function but using `open_dir_cmd` instead of `edit_cmd`
- Add `Ctrl+g c` global chord: open the demotui config directory (from `config::config_dir_path()`)
- Add `Ctrl+g m` global chord: open the clash config directory (from `CONFIG.cfg_file.basic.clash_config_dir`)
- Add `e` key binding in the Template tab to edit template files with the configured `edit_cmd`
- Update the help panel in `src/tui/widget/help.rs` to document the new global chords

## Capabilities

### New Capabilities
- `external-directory-open`: Spawn an external file manager command via the configured `open_dir_cmd` (with `%s` placeholder substitution) to open a directory path. Falls through to the platform default (`open` / `start`) via the config default value.

### Modified Capabilities
<!-- None -->

## Impact

- `src/functions/command.rs` — new `open_dir()` function
- `src/tui/app.rs` — new `global_chord` field, global chord layer, and dispatch logic
- `src/tui/widget/help.rs` — help text update for new shortcuts
- `src/tui/tab/files/template.rs` — new `Edit` action, key binding, and async callback
