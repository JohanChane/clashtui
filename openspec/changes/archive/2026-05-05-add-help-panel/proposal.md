## Why

Users need to discover available keyboard shortcuts without leaving the TUI or consulting external documentation. Currently the only shortcut visibility is the Which? panel during chord entry — there is no way to see all shortcuts for the active tab plus global shortcuts in one place. This creates a discoverability gap, especially for new users or infrequently used features.

## What Changes

- Add a new **Help Panel** overlay that displays when the user presses `?`
- Help panel renders as an overlay (similar to Which? panel) showing:
  - **Upper section**: all shortcuts for the currently focused tab (from `tab.shortcuts()`)
  - **Lower section**: all global shortcuts (tab switching, quit)
- Help panel intercepts all key events while open — pressing any key dismisses it (Esc or any other key)
- `?` key added to global key handler to toggle the help panel
- No changes to existing PopUp, Chord/Which, Tab, or Global layer logic

## Capabilities

### New Capabilities

- `help-panel`: Display a modal overlay listing all available keyboard shortcuts — focused tab shortcuts in the upper section, global shortcuts in the lower section. Toggle with `?`, dismiss with any key press.

### Modified Capabilities

- `layer-architecture`: Add Help as a new layer in the key routing chain, inserted after PopUp and before Chord/Which/Tab/Global. Help consumes all keys while active (dismisses on any key press), blocking all lower layers.

## Impact

- **`src/tui/app.rs`**: Add `help_visible: bool` field to App; add `?` to `handle_global_kv`; add help check + dismiss in `handle_key_event` after PopUp, before Chord; add help render block in `render()` after tabs, before Which?.
- **`src/tui/widget/help.rs`** (new): `render_help()` function that draws the bordered help panel overlay listing tab shortcuts and global shortcuts.
- **`src/tui/widget/mod.rs`**: Register `help` module.
