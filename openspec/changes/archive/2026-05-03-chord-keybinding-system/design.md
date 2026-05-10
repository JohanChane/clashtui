## Context

Currently demotui's keybinding is built around `mod_agent!` which generates `HashMap<KeyEvent, Action>` — a flat, one-to-one mapping. The TuiWidget trait exposes `handle_key_event(&mut self, kv: &KeyEvent)`. Each tab receives raw crossterm `KeyEvent` (including kind and state fields) and performs its own matching logic.

The routing order is: `PopUp (if active)` → `App global keys (tab switch, quit)` → `Tab`.

This design works for simple keybindings but has no support for:
- Multi-key sequences (e.g. `g g` for scroll-to-top, `Ctrl-w c` for close)
- Visual hint panel showing what keys are available after a prefix
- Clean separation between key matching and action execution

yazi's design provides a proven reference: `Chord` (Vec<Key> + Cmd), `Key` (code + modifier flags), layer-based matching, and a `Which` panel with progressive filtering.

## Goals / Non-Goals

**Goals:**
- Define a custom `Key` type (code + shift/ctrl/alt/super) decoupled from `KeyEvent`
- Define `Chord` as `on: Vec<Key>` + a runnable command
- Replace `mod_agent!` with a chord definition system per tab/layer
- Add a `Router` that matches incoming keys against chord tables
- Add a `Which` global state with progressive filtering and rendering
- Routing order: `Which (if active)` → `PopUp` → `App global` → `Layer/Tab`
- Tabs declare chords, not handle raw key events

**Non-Goals:**
- File-based keymap configuration (e.g. `keymap.yaml` loading)
- Changing PopUp behavior (Which is independent of PopUp)
- Adding modifier-only bindings as standalone actions (e.g. `Shift` alone)
- Multi-key across PopUp (keys go directly to PopUp, not through Router)

## Decisions

### 1. Key type: Custom struct vs raw KeyEvent

**Decision**: Custom `Key` struct with `code: KeyCode, shift: bool, ctrl: bool, alt: bool, super_: bool`.

**Rationale**: `crossterm::event::KeyEvent` includes `kind` (Press/Repeat/Release) and `state` (KeyEventState flags) that have no place in chord matching. For a sequence like `g g`, both presses should match regardless of repeat flags. The custom type normalizes cross-platform modifier behavior (e.g. `<S-a>` produces different modifier combinations on Unix vs Windows) and implements `Display` for rendering in the Which panel.

### 2. Chord structure: Vec<Key> + command vs plugin-like callback

**Decision**: `Chord { on: Vec<Key>, run: Vec<Cmd> }` with a command enum per layer.

**Rationale**: Reference yazi's model. Commands are enum variants (not dynamic callbacks) — this keeps chord definitions declarative and allows the Router to match and dispatch without calling into tab internals. The tab implements command execution. `Vec<Key>` allows sequences of any length; `Vec<Cmd>` allows a single chord to trigger multiple commands.

### 3. Routing: Router as front-end vs tab-internal

**Decision**: Router sits in `App`, matching against layer-specific chord tables before Tabs get involved.

**Rationale**: Centralized matching means Which state is managed in one place. Tabs become simpler: they declare chords (a registry) and implement command execution. The Router handles all the complexity of prefix matching, Which activation, and sequence accumulation.

### 4. Which integration: independent global state vs PopUp Msg

**Decision**: `Which` is an independent global state in `App`, not part of the PopUp queue.

**Rationale**: Which needs non-exclusive behavior — only matching keys are consumed, non-matching keys fall through to lower layers. The PopUp system is exclusive (active PopUp consumes all keys). Making Which a PopUp would require invasive changes to PopUp's key handling contract.

### 5. TuiWidget trait evolution

**Decision**: Remove `handle_key_event(&mut self, kv: &KeyEvent)` from TuiWidget. Add a method to register chords and another to execute commands.

```rust
trait TuiWidget {
    fn chords(&self) -> &[Chord];       // chord registry
    fn execute(&mut self, cmd: Cmd);    // command dispatch
    fn render(&mut self, f: &mut Frame, area: Rect);
    fn sync(&mut self);
}
```

**Rationale**: Tabs shouldn't care whether a command came from a single key or a 3-key sequence. The Router handles matching; Tabs only need to register chords and execute commands.

### 6. Layer system

**Decision**: Introduce a `Layer` enum. Initially: `App`, `Status`, `File` (matching existing tabs), plus `Which` as a transient routing-only layer.

```rust
enum Layer {
    App,    // global keys (tab switch, quit)
    Status, // StatusTab chords
    File,   // FileTab chords
    Which,  // transient — active during multi-key input
}
```

**Rationale**: Each UI context gets its own chord table. The `App` layer handles global keys (tab switching, quit). Tab layers handle content-specific chords. `Which` is the routing layer for multi-key continuation.

## Risks / Trade-offs

- **[Breaking] All existing Tab key handling must be rewritten**: Every `handle_key_event` implementation in StatusTab, FileTab, Profile, Template, and their sub-components must be converted to chord declarations + command execution. → Mitigated by doing it incrementally: Router + Which first, then migrate Tabs one at a time.
- **[Complexity] Router state machine**: The Router must handle: single-key match, prefix match → Which activation, Which continuation, Which timeout (if any), and fallback to lower layers. → Keep the state machine explicit with clear transitions, document in code.
- **[Performance] Chord matching is O(n) per key**: Each key press iterates over the active layer's chord list checking `on[0]` or `on[times]`. → Chord lists per layer are small (< 50 entries), this is negligible.
