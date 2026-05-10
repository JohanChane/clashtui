## Context

The TUI has a mature 4-layer key routing architecture (PopUp → Chord/Which → Tab → Global) but no way to surface available shortcuts to the user. The only existing shortcut visibility is the Which? panel, which only appears during chord entry and only shows chord candidates. Users cannot discover tab-specific or global shortcuts without reading source code or documentation.

Adding a Help panel requires inserting a new check between existing layers. The `layer-architecture` spec already defines the insertion pattern: add a struct field, one `if` check in `handle_key_event`, one conditional render, one sync call.

## Goals / Non-Goals

**Goals:**
- Show all shortcuts for the currently focused tab (from `tab.shortcuts()`)
- Show all global shortcuts (tab switching, quit, help toggle)
- Toggle help panel with `?` key from global shortcuts
- Block all lower-layer key handling while help is visible (dismiss on any key)
- Render as an overlay similar to the Which? panel (centered bordered block)
- Follow the layer architecture pattern defined in `layer-architecture` spec

**Non-Goals:**
- Customizable help content (no user-defined help pages)
- Search within help panel
- Context-sensitive help (e.g., showing help for a specific PopUp)
- Help for chord sequences (Which? already covers this)
- Scrolling if shortcuts overflow the screen (panels with many shortcuts should still be readable)

## Decisions

### Layer vs PopUp: Use a Minimal Struct (Layer Pattern)

**Decision**: Implement help as a `HelpPanel` struct wrapping a `bool`, following the same pattern as `PopUp` and `ChordHandler`. Put it in the widget module alongside the other layer types.

```rust
// src/tui/widget/help.rs
pub struct HelpPanel {
    visible: bool,
}

impl HelpPanel {
    pub fn is_active(&self) -> bool { self.visible }
    pub fn toggle(&mut self) { self.visible ^= true; }
    pub fn dismiss(&mut self) { self.visible = false; }
}
```

App field: `help: HelpPanel` (not `help_visible: bool`).

**Rationale**:
- **PopUp is for input gathering** — it uses `tokio::oneshot` channels for async result passing (Input, Confirm, InputMasked). Help is purely informational display with no return value.
- **PopUp adds complexity** — would require wrapping help state in a `Msg` impl, sending through the global `PAIR` channel, and managing trait object dispatch. For a simple toggle, this is unnecessary overhead.
- **Struct over bare bool** — a struct matches the pattern of every other layer (`PopUp`, `ChordHandler`), costs 0 runtime overhead, and allows adding fields later (scroll, search, dismiss timer) without changing call sites.

### Key Binding: `?` in Global Layer

**Decision**: Add `KeyCode::Char('?')` to `handle_global_kv` (layer 3). It calls `self.help.toggle()`.

**Rationale**:
- `?` is the standard help key in vim, lazygit, and other TUI tools
- Doesn't conflict with existing bindings (no tab uses `?`)
- Placing it in Global means it works from any tab
- Not all tabs have shortcuts, so `?` shouldn't be a tab-specific key

### Insertion Position: Between PopUp and Chord

**Decision**: Insert the help check at the top of `handle_key_event`, immediately after PopUp and before Chord.

```rust
fn handle_key_event(&mut self, kv: &KeyEvent) {
    if self.popup.check() {
        self.popup.handle_key_event(kv);
        return;  // PopUp (layer 0) — unchanged
    }
    if self.help.is_active() {
        self.help.dismiss();  // any key dismisses
        return;
    }
    // ... Chord (layer 1), Tab (layer 2), Global (layer 3) — unchanged
}
```

**Rationale**:
- **PopUp remains highest priority** — if a confirmation dialog is open, help can still be behind it or already toggled off. PopUp is always most urgent.
- **Help blocks all lower layers** — we want to prevent accidental actions while reading help. Pressing any key should dismiss help, not trigger an action.
- **Respects existing routing contract** — the `return` pattern matches how PopUp works: consume the key and stop routing.
- **No change to Chord, Tab, or Global logic** — the `return` is a simple early exit before those layers are reached.

### Alternative: Inserting After Tab, Before Global

This was considered but rejected. If help is positioned after Tab (layer 2) and before Global (layer 3), then tab keys would still fire while help is open. For example, pressing `j` (move down) would both scroll the list AND dismiss help. This is confusing — the user sees help, presses a key to dismiss it, and accidentally triggers a tab action. The consistent behavior is: help is a "blocking overlay" that consumes all keys.

### Rendering: Center-Overlay with Two Sections

**Decision**: Render as a centered bordered popup with two sections:
- **Upper section** ("Tab Shortcuts"): all `(key, desc)` pairs from `tab.shortcuts()`, in 2 columns
- **Lower section** ("Global Shortcuts"): hardcoded list of global key bindings

Use `Clear` widget to blank the area before rendering (same pattern as Which? panel).

Width: 60 columns. Height: dynamic based on content. Centered on screen.

**Rationale**:
- Two sections clearly separate scope (what works in this tab vs everywhere)
- 2-column layout fits most shortcut tables without being too wide
- `Clear` ensures the overlay doesn't have visual artifacts from content behind it
- Reusing `key_event_to_str()` from chord.rs for consistent key name display

### Rendering Position: After Which? Panel

**Decision**: In `render()`, check and render help panel after `render_which()`:

```rust
self.tabs[self.tab_index as usize].render(f, chunks[1]);

if self.chord.is_active() {
    self.render_which(f);  // Which? (shown during chord entry)
}
if self.help.is_active() {
    self.render_help(f);   // Help (shown on '?' press)
}
if self.popup.check() {
    self.popup.render(f, Default::default());  // PopUp (always topmost)
}
```

**Rationale**: Which? and Help are mutually exclusive (chord mode is active OR help is visible, never both), but placing Help after Which? ensures Which? always renders on top if both were somehow active. PopUp must remain last (topmost visual layer) since it represents urgent modal dialogs.

## Risks / Trade-offs

- **Tab with many shortcuts may overflow screen**: Some tabs (Proxies: 18 entries, Connections: 15) produce tall help panels. The design uses a dynamic height calculation with a sensible minimum screen size assumption (24 rows). If a terminal is very small, the help panel may clip — acceptable trade-off vs adding scrolling complexity.
- **`?` conflicts with user keymap**: User-defined `keymap.yaml` could theoretically map `?` to a tab action via `agent_init`. However, `?` is resolved in the Global layer (always runs after Tab), so the tab action would fire first AND the Global layer would also toggle help. To avoid this, the `?` check should also handle the case where a tab already consumed it — solved by placing help check before Tab layer in routing.
