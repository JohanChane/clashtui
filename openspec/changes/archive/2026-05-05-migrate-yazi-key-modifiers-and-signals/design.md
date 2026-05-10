## Context

demotui currently uses `crossterm::KeyEvent` directly for all key matching. The `mod_agent!` macro only accepts `KeyCode` and generates shortcuts with `KeyModifiers::empty()`. There is no signal handling — Ctrl-C in raw mode is silently dropped, and the process has no response to SIGTERM. The `chord-keybinding` spec already defines the target `Key` struct and `Chord` struct for future routing, but neither is implemented.

This change implements the `Key` struct (from `chord-keybinding` spec) and adds signal handling, while keeping the existing layer architecture intact. The full Chord/Layer routing from `chord-keybinding` is deferred to a future change.

## Goals / Non-Goals

**Goals:**
- Define and use a custom `Key` struct with explicit modifier booleans, with cross-platform normalization from crossterm's `KeyEvent`
- Update `KeyCombo`, `mod_agent!`, `shortcuts()`, and `dispatch_shortcut()` to use `Key` instead of `crossterm::KeyEvent`
- Support modifier syntax in `mod_agent!` macros: `<C-q>`, `<A-x>`, `<S-Up>`, `<D-h>`, `<C-S-x>`, and bare chars with shift-detection
- Add signal handling for SIGQUIT, SIGHUP, SIGTERM (graceful quit), SIGTSTP (suspend), SIGCONT (resume), with SIGINT intentionally ignored
- Add `<C-c>` as a default keybinding for close/quit
- Keep the existing 4-layer routing (PopUp → Chord → Tab → Global) unchanged

**Non-Goals:**
- Full Chord/Layer routing from `chord-keybinding` spec (Chord struct, per-layer chord registry, Router, Cmd execution) — deferred
- Changing the YAML keymap format (serialization adaptation is handled by the `Key` type's serde implementation)
- Adding custom key display formatting beyond `Display` on `Key`
- Support for all crossterm `KeyCode` variants in `FromStr` — only existing variants used in the codebase

## Decisions

### Decision 1: Key struct design — flat booleans, not bitmask

**Chosen**: `{ code: KeyCode, shift: bool, ctrl: bool, alt: bool, super_: bool }`

**Rationale**: Proved in yazi. Flat booleans are simpler to pattern-match in macros and keymaps than bitflag operations. The `Display` impl can easily order modifiers (D-C-A-S-). `From<KeyEvent>` provides cross-platform normalization.

**Alternative**: `{ code: KeyCode, modifiers: KeyModifiers }` — crossterm's own type. Rejected because it doesn't normalize platform differences (Unix vs Windows shift behavior on non-alpha keys), and requires bitflag ops in every comparison.

### Decision 2: KeyCombo wraps Vec<Key> (not Vec<KeyEvent>)

**Chosen**: `KeyCombo(Vec<Key>)` replaces `KeyCombo(Vec<KeyEvent>)`.

**Rationale**: `Key` is the normalized, user-facing type. All matching and display logic should use `Key`. The conversion from `KeyEvent` happens once at the event source (in `Signals::handle_term`), then everything downstream uses `Key`.

### Decision 3: Signal handler in a spawned tokio task, communicating via the existing global TX channel

**Chosen**: A new `Signals` struct spawns a `tokio::spawn` task using `signal_hook_tokio`. The task converts OS signals to the existing `Event` enum variants (`Event::Quit`), sent via `TX.send()`. Stop/resume control uses a separate `mpsc` channel between `Signals` and the spawned task.

**Rationale**: Integrates with the existing event loop without restructuring. The spawned task can use `tokio::select!` (biased) to prioritize stop logic over signal processing over terminal events. The main event loop in `App::serve()` already handles `Event::Quit` for graceful shutdown.

```
Signals::start() spawns tokio task:
  tokio::select! { biased;
    rx.recv()         → stop/resume control (toggle EventStream)
    sys.next()        → OS signal → Event::Quit / suspend logic
    EventStream.next() → KeyEvent → Event::Key(Key::from(ev))
  }
```

### Decision 4: SIGINT is intentionally ignored at the OS level

**Chosen**: The signal handler explicitly ignores SIGINT. In raw mode, Ctrl-C arrives as `KeyEvent{ Char('c'), CTRL }` and is routed through the keymap system.

**Rationale**: This is how yazi does it, and it's the correct design for terminal applications. `enable_raw_mode()` disables the terminal's `ISIG` flag, so the TTY never translates `^C` to SIGINT. Even if SIGINT arrives via `kill -INT` from outside, we want the keybinding system to handle Ctrl-C (close tab / quit gracefully), not an abrupt signal handler.

### Decision 5: KeyCombo comparison — compare Key codes + modifiers

**Chosen**: `Key` derives `PartialEq, Eq, Hash`. KeyCombo equality is derived from `Vec<Key>` equality.

**Rationale**: This means `<C-c>` and `c` are different keys (different modifiers). Simple and correct. No need for a custom `Eq` impl.

## Risks / Trade-offs

- **[Risk] Breaking keymap.yaml format**: Existing user keymaps use serialized `crossterm::KeyEvent`. The new `Key` type needs a different serialization format. **Mitigation**: The `Key` Display format (`<C-S-x>` or bare `q`) becomes the YAML key format. Provide clear migration path in CHANGELOG.

- **[Risk] mod_agent! macro complexity**: The macro already has complex muncher rules. Adding modifier parsing increases complexity. **Mitigation**: The modifier prefix parsing is a simple `starts_with('<')` check — if the first token is a `<X-ch>` form, parse modifiers; otherwise fall through to existing `KeyCode` matching.

- **[Risk] Signal handling on Windows**: Windows doesn't have POSIX signals. **Mitigation**: Gate signal handling behind `#[cfg(unix)]`. On Windows, SIGINT is already handled by the console host, and the app simply gets a `KeyEvent` for Ctrl-C.

## Open Questions

- Should `<C-c>` close the tab or quit immediately? Yazi uses `mgr:close` (close tab, quit if last). demotui does not have multi-tab closure semantics yet, so `<C-c>` should simply quit for now.
