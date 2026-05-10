## 1. ChordHandler module

- [x] 1.1 Create `src/tui/widget/chord.rs` with `ChordHandler` struct (`pressed`, `candidates` fields)
- [x] 1.2 Implement `ChordHandler::default()` (empty pressed, empty candidates)
- [x] 1.3 Implement `ChordHandler::is_active()` → `!pressed.is_empty()`
- [x] 1.4 Implement `ChordHandler::handle()` — entry point: dispatch to `continue_()` or `check_init()` based on `is_active()`
- [x] 1.5 Implement `continue_()` — filter candidates, auto-dispatch on exact match or single candidate, Esc/cancel, non-match cancels and consumes
- [x] 1.6 Implement `check_init()` — check single-key shortcuts first (via agent lookup), then chord prefix detection
- [x] 1.7 Move `key_event_to_str()` from `src/tui/app.rs` to `src/tui/widget/chord.rs` (used in panel rendering)
- [x] 1.8 Register chord module in `src/tui/widget/mod.rs`

## 2. TuiTab trait + macros — interface update

- [x] 2.1 Update `TuiTab` trait: `shortcuts()` returns `&[(KeyCombo, &str)]` (was `Vec`), `dispatch_shortcut()` returns `()` (was `bool`)
- [x] 2.2 Update `newtype_tab!` macro: delegate to inner type with new signatures
- [x] 2.3 Update `enum_dispatch!` macro: match arms with new signatures
- [x] 2.4 Update `new_type_impl_tuiwidget!` if it touches shortcut methods

## 3. Tab\<C\> — zero-alloc shortcuts

- [x] 3.1 Change `Tab::shortcuts()` return type to `&[(KeyCombo, &str)]`
- [x] 3.2 Implement `OnceLock`-cached display shortcuts via `C::all_shortcuts()` clone-once pattern
- [x] 3.3 Change `Tab::dispatch_shortcut()` return type to `()` (no bool needed)
- [x] 3.4 Handle esc in `dispatch_shortcut` — find action from `all_shortcuts()` by matching `&**s == seq` and call `content.handle_key_event(key, tasks, state)`

## 4. DualTab\<C1,C2\> — zero-alloc shortcuts

- [x] 4.1 Change `DualTab::shortcuts()` return type to `&[(KeyCombo, &str)]`
- [x] 4.2 Implement dual `OnceLock` caches (one per content type), select based on `is_focus_on_c1`
- [x] 4.3 Change `DualTab::dispatch_shortcut()` return type to `()`
- [x] 4.4 Ensure focus switching (Left/Right) works correctly alongside chord dispatch

## 5. App routing — remove WhichState, add ChordHandler

- [x] 5.1 Remove `WhichState` struct from `App`
- [x] 5.2 Remove `handle_which()` method
- [x] 5.3 Add `chord: ChordHandler` field to `App`
- [x] 5.4 Rewrite `handle_key_event` routing: PopUp → chord.handle(...) → Tab → Global
- [x] 5.5 The `chord.handle()` call passes closures: `shortcuts()` and `dispatch_shortcut()` from active tab
- [x] 5.6 Update `render`: check `self.chord.is_active()` and call `render_which` if true
- [x] 5.7 Adapt `render_which` to read `self.chord.pressed` and `self.chord.candidates` directly
- [x] 5.8 Remove import of `key_event_to_str` (now in chord.rs); import from new location

## 6. Profile/Template — verify mod_agent! compatibility

- [x] 6.1 Verify `mod_agent!` macro syntax unchanged — `([...], Key, desc)` format still works
- [x] 6.2 Verify Profile's `mod_agent!` call compiles with new Tab interface
- [x] 6.3 Verify Template's `mod_agent!` call compiles with new Tab interface
- [x] 6.4 Verify Status's `Key` enum still works (empty, Copy)

## 7. Unit tests

- [x] 7.1 Add `ChordHandler` unit tests: init from key, continue matching, continue non-matching (cancel + consume), Esc cancel, exact match dispatch, single candidate dispatch, single-key priority over chord prefix
- [x] 7.2 Update existing App unit tests for new routing structure
- [x] 7.3 Add unit test: verify non-matching key during chord is consumed (chord cancelled, key not propagated)

## 8. Integration & verification

- [x] 8.1 `cargo check` — type errors across all modules
- [x] 8.2 `cargo test` — all tests pass (11 + new chord tests)
- [x] 8.3 Manual: single-key shortcuts dispatch transparently (no panel)
- [ ] 8.4 Manual: `g` in Profile shows Which panel, `g` again jumps to top
- [ ] 8.5 Manual: `g` then `1` cancels chord, Which panel closes, key consumed (no tab switch)
- [ ] 8.6 Manual: `g` then `q` cancels chord, Which panel closes, key consumed (no quit)
- [ ] 8.7 Manual: Esc during chord cancels but does nothing else
- [ ] 8.8 Manual: DualTab focus switching (Left/Right) works normally
