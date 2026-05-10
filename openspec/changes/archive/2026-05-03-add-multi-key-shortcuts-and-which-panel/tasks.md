## 1. Extend mod_agent! macro for unified shortcuts

- [x] 1.1 Add `shortcuts()` generation to `mod_agent!` macro: accept `(KeyCode, Key, desc)` 3-tuples for single-key and `(KeyCode, KeyCode, Key, desc)` 4-tuples for chords; produce `fn shortcuts() -> &'static [(Vec<KeyEvent>, Key, &'static str)]` as a `OnceLock`-protected static
- [x] 1.2 Update Profile's `mod_agent!` invocation to use the new syntax (keep all existing bindings, add `g g → GoTop`, `g e → GoEnd` as example chords)
- [x] 1.3 Update Template's `mod_agent!` invocation to use the new syntax (no chords, just convert existing single-key entries)
- [x] 1.4 Run `cargo check` to verify the macro compiles

## 2. Add shortcuts + dispatch to Tab/DualTab

- [x] 2.1 Add `shortcuts(&self) -> Vec<(Vec<KeyEvent>, &'static str)>` and `dispatch_shortcut(&mut self, seq: &[KeyEvent]) -> bool` methods to `Tab<C>` struct (accessible on the concrete struct, not through `TuiWidget`)
- [x] 2.2 Add `shortcuts()` and `dispatch_shortcut()` methods to `DualTab<C1,C2>` struct, using `is_focus_on_c1` to delegate to the focused pane's shortcuts
- [x] 2.3 Extend `enum_dispatch!` macro to generate dispatch arms for `shortcuts()` and `dispatch_shortcut()` on the `Tab` enum
- [x] 2.4 Run `cargo check` to verify the new methods compile

## 3. Implement Which layer on App

- [x] 3.1 Add `which: Option<WhichState>` field to `App` struct
- [x] 3.2 Implement `handle_which(&mut self, kv: &KeyEvent) -> bool`: inactive → check shortcuts for single-key (dispatch) or chord prefix (activate); active → filter candidates, handle Esc, auto-dispatch
- [x] 3.3 Update `App::handle_key_event` routing: `handle_which()? → PopUp → handle_global_kv() → Tab`
- [x] 3.4 Run `cargo check` to verify routing logic compiles

## 4. Render which panel

- [x] 4.1 Implement `render_which()` — bordered floating box with title " Which? ", candidates in 1-2 columns using ratatui `Layout::horizontal`, `Clear` widget for overlay
- [x] 4.2 Each candidate row renders as: `key  description` where key is formatted from `KeyEvent` (e.g., `g`, `h`, `<Enter>`)
- [x] 4.3 Add which panel rendering to `App::render()` — after tab content, before popup, guarded by `self.which.is_some()`

## 5. Verification

- [x] 5.1 `cargo check` — entire project compiles
- [x] 5.2 `cargo test` — all existing tests pass
- [x] 5.3 Manual check: Profile tab `g g` goes to top, `g e` goes to end, which panel appears/disappears correctly
