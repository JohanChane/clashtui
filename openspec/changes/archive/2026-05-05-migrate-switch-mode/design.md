## Context

demotui's SrvCtl tab (`src/tui/tab/srvctl.rs`) provides a list of service-control operations: Stop Service, Start Service, Set Permission, and Fix File Permissions. It uses the `Tab<C>` widget framework where content is defined by `SrvCtlContent` implementing `TabContent` with `State = ListState`.

clashtui's equivalent tab has a "SwitchMode" operation that opens a sub-selector. When the user selects a mode (Rule/Direct/Global), the clash API is patched via `PATCH /configs` with `{"mode": "<mode>"}`.

demotui already has:
- `Mode` enum in `src/functions/restful/config_struct.rs` with `serde(rename_all = "lowercase")` — its `Display` impl produces the lowercase strings clash expects
- `LogLevel` enum in the same file (Silent/Error/Warning/Info/Debug) — also has `serde(rename_all = "lowercase")` and `Display` producing lowercase strings
- `config::patch()` in `src/functions/restful.rs` that issues `PATCH /configs` with a JSON payload
- `InputMasked` / `Confirm` popup infrastructure for user feedback

## Goals / Non-Goals

**Goals:**
- Add "Switch Mode" and "Switch Log Level" as selectable operations in the SrvCtl tab's main list
- Render a centered selector overlay when either is activated
- Each selector shows the applicable options (Mode: Rule/Direct/Global; LogLevel: Silent/Error/Warning/Info/Debug)
- Navigate selectors with Up/Down, confirm with Enter, cancel with Esc
- On confirmation, spawn an async task that calls `config::patch()` with the appropriate JSON payload and shows the result

**Non-Goals:**
- The FileTab or other tabs — this is SrvCtl-only
- Exposing mode/log-level switch as a global shortcut or chord
- Fetching and displaying the current mode or log level before the selector opens
- CLI-only / non-TUI switching

## Decisions

### 1. Add selector state directly in SrvCtlContent

**Decision**: Add two pairs of fields to `SrvCtlContent`: `(mode_selector_state, mode_selector_visible)` and `(log_level_selector_state, log_level_selector_visible)`. Also add pre-populated `Vec<Mode>` and `Vec<LogLevel>`. All selector state fields default to hidden/unselected via `#[derive(Default)]`.

**Rationale**: The `Tab<C>` framework provides a single `state: C::State` which is already `ListState` for the main operation list. Changing the State type would affect all TabContent trait method signatures. Keeping selectors as separate fields in the content struct is simpler and isolates the change. Two separate pairs avoid the complexity of a generic selector abstraction for what is only two instances.

### 2. Add `SwitchMode` and `SwitchLogLevel` to SrvCtlOp enum

**Decision**: Add both variants to `SrvCtlOp` with display strings "Switch Mode" and "Switch Log Level". Include them in `SrvCtlOp::all()`.

### 3. Key routing: same key bindings, context-dependent behavior

**Decision**: Add `Esc` to the `SrvCtlKey` enum (mapped to `KeyCode::Esc`). When either selector is visible, `handle_key_event` delegates MoveUp/MoveDown/Execute to that selector and Esc closes it. When neither is visible, all keys route to the main ops list as before. The `Execute` handler checks `mode_selector_visible` first, then `log_level_selector_visible`, then dispatches to the main list.

**Alternatives considered**:
- A separate key enum per selector — adds complexity for simple sub-navigation
- Using PopUp-like one-shot pattern — not appropriate because the user navigates interactively (not just a single input), and PopUp would block the event loop

### 4. API call: use existing config::patch()

**Decision**: In the async task spawned on confirmation, call:
```rust
// Mode switch
let payload = serde_json::json!({"mode": mode.to_string()}).to_string();
crate::functions::restful::config::patch(payload)

// Log level switch
let payload = serde_json::json!({"log-level": level.to_string()}).to_string();
crate::functions::restful::config::patch(payload)
```

Note the clash API uses kebab-case `"log-level"` for the log level field. Both `Mode` and `LogLevel` `Display` impls produce the lowercase strings the API expects.

### 5. Rendering: centered overlay with Clear background

**Decision**: When a selector is visible, render a second list in a centered rectangle calculated as `centered_rect(60, 30, f.size())` (or similar proportions). Use `ratatui::widgets::Clear` to erase the area behind the overlay. Only one selector can be visible at a time, so no overlap conflict.

## Risks / Trade-offs

- **[Esc while in selector]**: The Tab widget's `handle_key_event` dispatches to content, but the outer `App::handle_global_kv` handler at layer 3 (Global) also checks Esc. Esc already returns `false` from global handler (doesn't consume), so it will reach the tab's content handler. Verify this doesn't interfere.
  → **Mitigation**: Test that Esc closes the selector without also triggering a global action.

- **[Selector stays open across tab switches]**: If user switches tabs while a selector is visible, the state persists but won't render (because the tab is not active). When switching back, the selector could be confusingly still open.
  → **Mitigation**: This is acceptable — it mirrors clashtui behavior and is intuitive since tab switching is infrequent.

- **[No feedback on current value]**: Selectors don't show which mode or log level is currently active before selection.
  → **Mitigation**: Out of scope for this change (non-goal). Can be added later as a separate enhancement.

- **[Log level API field is kebab-case]**: The clash API expects `"log-level"` in the JSON payload, while `Mode` uses `"mode"`. The `serde(rename_all = "lowercase")` on `LogLevel` produces `"silent"` etc., but the JSON key must be `"log-level"`.
  → **Mitigation**: Use `serde_json::json!({"log-level": ...})` explicitly in the async task rather than deriving from the field name.
