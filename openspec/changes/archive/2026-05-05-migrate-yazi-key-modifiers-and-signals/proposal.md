## Why

demotui currently uses `crossterm::KeyEvent` directly — all built-in shortcuts ignore modifiers (`KeyModifiers::empty()`), `Ctrl-C` is silently dropped, and the process has no graceful shutdown path for SIGTERM/SIGHUP. This makes the key system fragile (Shift-A vs A are indistinguishable in defaults), precludes Ctrl-based shortcuts, and leaves the app unkillable via standard signals. Yazi's design solves all of this with a minimal, proven architecture.

## What Changes

- **New `Key` struct** — `{ code, shift, ctrl, alt, super_ }` with cross-platform normalization (yazi-proven `From<KeyEvent>`, `FromStr`, `Display`). Replaces raw `KeyEvent` in key matching.
- **New `KeyCombo`** — wraps `Vec<char>` (the key code) rather than `Vec<KeyEvent>`, using the new `Key` for matching.
- **Enhanced `mod_agent!`** — accepts modifier prefixes (`<C-q>`, `<A-x>`, `<S-Up>`) in key definitions, generates approriate `Key` instances.
- **Signal handling** — new `Signals` module catches SIGQUIT/SIGHUP/SIGTERM for graceful shutdown, SIGTSTP for suspend, SIGCONT for resume. SIGINT is intentionally ignored (Ctrl-C arrives as keyboard event in raw mode).
- **Ctrl-C binds to `close`** — `<C-c>` added as a default keybinding that closes the current tab (or quits if last tab), mirroring yazi's `mgr:close` behavior.
- **BREAKING**: `KeyCombo`, `shortcuts()`, `dispatch_shortcut()`, and the YAML keymap format change to use the new `Key` type. User-provided `keymap.yaml` files must be updated.

## Capabilities

### New Capabilities
- `signal-handling`: Graceful shutdown on SIGQUIT/SIGHUP/SIGTERM, suspend/resume on SIGTSTP/SIGCONT, with SIGINT intentionally ignored (delegated to keyboard input in raw mode).

### Modified Capabilities
- `shortcut-bindings`: `mod_agent!` macro syntax extends to accept the new `Key` type (with modifiers). `shortcuts()` return type changes from `Vec<(Vec<KeyEvent>, &str)>` to `Vec<(Vec<Key>, &str)>`. `dispatch_shortcut()` signature updates to use `&[Key]` instead of `&[KeyEvent]`.

## Impact

- **Affected code**: `src/tui/agent.rs`, `src/tui/app.rs`, `src/tui/tab/mod.rs` (mod_agent!), `src/tui/widget/chord.rs`, `src/tui/widget/tab.rs`, all tab `mod_agent!` invocations, `src/tui/utils.rs` (signal setup)
- **New files**: `src/tui/key.rs` (Key struct), `src/tui/signals.rs` (signal handling)
- **New dependency**: `signal-hook-tokio` (with `futures-v0_3` feature)
- **YAML keymap format**: Keys currently serialized as crossterm `KeyEvent` must become the new `Key` format
