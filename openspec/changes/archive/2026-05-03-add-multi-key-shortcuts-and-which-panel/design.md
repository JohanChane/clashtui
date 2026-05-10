## Context

Demotui uses a `mod_agent!` macro per tab to generate a static `HashMap<KeyEvent, Key>` for single-key dispatch. Each tab's `TryFrom<&KeyEvent>` converts a key event into a tab-specific `Key` enum variant, which the tab's `handle_key_event` method uses to run actions. The event loop routes keys: PopUp → App → Tab.

Yazi's which panel intercepts ALL keys: single-key chords execute immediately, multi-key prefixes open the which panel with filtered candidates. This design removes the need for separate routing paths — one layer handles everything.

## Goals / Non-Goals

**Goals:**
- Which layer at the top of routing intercepting all keys before PopUp/App/Tab
- Focused panel exposes `shortcuts()` (all bindings as key-sequence + description) and `dispatch_shortcut(&[KeyEvent])` (resolve sequence to action and execute)
- `mod_agent!` generates a unified `shortcuts()` static for each pane
- Single-key shortcuts dispatch invisibly (no panel shown)
- Multi-key prefixes activate the which panel overlay
- Incremental filtering: each subsequent key narrows candidates; auto-execute when 1 remains or exact match found
- Esc dismisses the which panel
- DualTab delegates to the focused pane via `is_focus_on_c1`

**Non-Goals:**
- Configurable shortcuts loaded from `keymap.yaml` (all shortcuts are hardcoded)
- N-key chords beyond 2 (structure supports N, but only 2-key chords used initially)
- Custom theming for the which panel
- Modifier key chords (Ctrl/Alt/Shift — possible in model but not initial usage)

## Decisions

### 1. Which layer is the first routing stage

```
KeyEvent → Which → PopUp → App (global) → Tab (fallback)
```

When Which is active, it consumes the key and no other layer sees it. When inactive, it checks if the key is a shortcut prefix; if not, the key falls through to PopUp → App → Tab.

The Tab's `handle_key_event` becomes a fallback for unknown keys. All known shortcuts are intercepted and dispatched by Which.

**Reason:** Single routing path, no duplicated dispatch logic between Which and Tab.

### 2. `mod_agent!` generates a unified `shortcuts()` static

Current macro generates:
```rust
fn agent() -> &'static HashMap<KeyEvent, Key>;
fn agent_init(map: HashMap<KeyEvent, Key>);
```

New macro adds:
```rust
fn shortcuts() -> &'static [(Vec<KeyEvent>, Key, &'static str)];
```

Where each entry contains the full key sequence (length 1 for single-key, 2+ for chords), the action, and a description. The existing `agent()` and `agent_init()` are preserved for keymap.yaml loading (single-key only).

### 3. Macro syntax extension

```rust
mod_agent!(
    Key,
    [
        // single-key: (KeyCode, Key, description)
        (KeyCode::Enter, Key::Action(Action::Apply), "Apply profile"),
        (KeyCode::Char('i'), Key::Action(Action::Add), "Add profile"),
        // chord: (prefix_keycode, suffix_keycode, Key, description)
        (KeyCode::Char('g'), KeyCode::Char('g'), Key::Action(Action::GoTop), "Go to top"),
        (KeyCode::Char('g'), KeyCode::Char('e'), Key::Action(Action::GoEnd), "Go to end"),
    ]
);
```

The first form (2-tuple) creates a single-key shortcut. The second form (4-tuple) creates a 2-key chord. The macro auto-inserts the key events with `KeyEventKind::Press`.

**Alternative:** Separate `mod_chords!` macro. Rejected — doubles boilerplate and splits related bindings.

### 4. Focused panel interface

Two methods added to `TuiTab` (not `TuiWidget`, to keep it tab-specific):

```rust
pub trait TuiTab: super::TuiWidget {
    fn title(&self) -> &'static str;

    fn shortcuts(&self) -> Vec<(Vec<KeyEvent>, &'static str)>;
    fn dispatch_shortcut(&mut self, seq: &[KeyEvent]) -> bool;
}
```

Both are implemented by `Tab<C>`, `DualTab<C1,C2>`, and delegated through the `Tab` enum dispatch.

- `shortcuts()` returns `(key_sequence, description)` — not including the `Key` value (which is type-specific). Descriptions needed for which panel display, sequences needed for matching. The `Key` value is needed later for `dispatch_shortcut`.
- `dispatch_shortcut(seq)` — the container matches `seq` against the focused pane's `shortcuts()` (the full static including the `Key`), resolves to the `Key` enum, and calls `handle_key_event(key, ...)`. Returns `true` if dispatched, `false` if sequence not found.

**Important:** `shortcuts()` returns `Vec<(Vec<KeyEvent>, &str)>` (without `Key` variant) since the `Tab` enum dispatches to different concrete types. The full `ShortcutEntry` (with `Key`) is only available inside `dispatch_shortcut()` where the concrete type is known.

Actually, simplier: store the full data as a static `[(Vec<KeyEvent>, Key, &str)]`, and `shortcuts()` maps it to `Vec<(Vec<KeyEvent>, &str)>`.

### 5. `new_type_impl_tuiwidget!` extension

Since `shortcuts()` and `dispatch_shortcut()` are on `TuiTab` (not `TuiWidget`), the `newtype_tab!` macro already delegates `TuiTab` impls. The `new_type_impl_tuiwidget!` macro only delegates `TuiWidget` (handle_key_event, render, sync) — it doesn't need changes.

But the `Tab` enum's auto-generated dispatch needs updating in `enum_dispatch!` to include the new methods.

**Decision:** Extend `enum_dispatch!` macro to generate `shortcuts()` and `dispatch_shortcut()` arms. These dispatch to the corresponding inner type's implementation.

### 6. Which panel state

```rust
struct WhichState {
    pressed: Vec<KeyEvent>,
    candidates: Vec<(Vec<KeyEvent>, &'static str)>,
}
```

- `pressed`: accumulated prefix keys already pressed (always at least 1)
- `candidates`: filtered list of remaining shortcuts that match `pressed` as a prefix

### 7. Which panel matching algorithm

```
On key press when which is INACTIVE:
  1. Get shortcuts from focused panel
  2. Filter: shortcuts whose seq[0] == key
  3. For each match:
     a. If seq.len() == 1: single-key shortcut → dispatch immediately, return consumed
     b. If seq.len() > 1: chord prefix — add to candidates list
  4. If candidates non-empty: activate which panel with pressed = [key], candidates
  5. If nothing matched: return not consumed (fall through to PopUp/App/Tab)

On key press when which is ACTIVE:
  1. Esc → close which, return consumed
  2. Append key to pressed
  3. Filter candidates: retain those where seq[pressed.len() - 1] == key
  4. If candidates.len() == 0: close which, return consumed
  5. If candidates.len() == 1: dispatch the sole candidate, close which
  6. If any candidate has exact seq.len() == pressed.len(): dispatch it, close which
  7. Else: keep which open with filtered candidates
```

### 8. Which panel rendering

A floating bordered box rendered as an overlay on top of tab content. Layout:
- Title: " Which? "
- Content: candidates arranged in columns
- Each row per candidate: `key  description`
- 1 column for ≤4 candidates, 2 columns for 5+
- Rendered AFTER tab content in `App::render()`
- Uses `Clear` widget before rendering for overlay effect

## Risks / Trade-offs

- **Risk:** Which layer intercepting all keys means unknown keys no longer reach the Tab for custom handling. → Mitigation: The `dispatch_shortcut` → `Tab::handle_key_event` still works; unknown keys that don't match any shortcut fall through normally.
- **Risk:** DualTab shortcuts depend on `is_focus_on_c1` which can change mid-chord if the prefix key also triggers a pane switch. → Mitigation: Chords don't use Switch keys as prefixes. The prefix check runs before the switch action.
- **Risk:** `shortcuts()` allocates a Vec every call. → Mitigation: Acceptable — it's called once per key press and the Vec is tiny (<20 entries).
