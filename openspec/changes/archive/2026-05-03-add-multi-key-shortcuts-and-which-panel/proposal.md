## Why

The current key binding system only supports single-key shortcuts (e.g., `i` for add, `d` for delete). As the number of actions grows, the available single-letter keys become scarce. Multi-key sequences (e.g., `g g` for go-to-top, `m s` for linemode-size) dramatically expand the action space without requiring modifier-heavy chords. Yazi's "which" panel elegantly solves the discoverability problem: pressing a prefix key immediately shows all possible completions, so users don't need to memorize sequences.

## What Changes

- **Unified Which layer**: A new key-routing layer at the top of `App::handle_key_event` that intercepts ALL keys. It queries the focused panel's shortcuts (single-key + multi-key chords), dispatches single-key actions transparently, and opens a which panel for multi-key prefixes
- **`shortcuts()` / `dispatch_shortcut()` on each panel**: The focused panel exposes its full shortcut table and a dispatch method, so the Which layer can decide what to show and what to execute without knowing tab-specific `Key` types
- **`mod_agent!` generates unified `shortcuts()`**: The macro produces a single static `&[(Vec<KeyEvent>, Key, &str)]` covering both single-key bindings (vec of length 1) and multi-key chords (vec of length 2+)
- **Which panel rendering**: A floating overlay rendering candidate shortcuts as `keys → description` in 1-2 columns, auto-dismissed on match/cancel
- **No configuration**: All shortcuts are hardcoded via `mod_agent!`; no keymap.yaml support for chords

## Capabilities

### New Capabilities

- `shortcut-bindings`: Define unified per-panel shortcut tables (single-key + multi-key chords) via `mod_agent!`, and expose them through `shortcuts()` / `dispatch_shortcut()` methods on the focused panel
- `which-layer`: A key-routing layer at the top of the App event loop that handles all shortcut dispatch — single-key actions execute immediately and transparently, multi-key prefixes activate a which panel for completion selection

### Modified Capabilities

<!-- No existing specs to modify -->

## Impact

- `src/tui/tab/mod.rs` — `mod_agent!` extended to generate `shortcuts()` alongside existing `agent()`; `TuiTab` trait extended with `shortcuts()` / `dispatch_shortcut()`
- `src/tui/widget/tab.rs` — `Tab<C>` implements `shortcuts()` / `dispatch_shortcut()` for single-pane tabs
- `src/tui/widget/dualtab.rs` — `DualTab<C1,C2>` implements `shortcuts()` / `dispatch_shortcut()` delegating to focused pane via `is_focus_on_c1`
- `src/tui/app.rs` — New `WhichState` struct, `handle_which()` method, updated routing (`Which → PopUp → App → Tab`), which panel rendering
- `src/tui/widget/mod.rs` — `new_type_impl_tuiwidget!` extended to delegate new methods
- `src/tui/tab/files/profile.rs` — Example chords added
