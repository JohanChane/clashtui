## Why

The `Input` and `InputMasked` widgets corrupt text when multi-byte UTF-8 characters (Chinese, Japanese, Korean, emoji) are inserted anywhere except at the end of the buffer. `enter_char()` treats the cursor as a byte index, but it only increments by 1 per character — inserting into a string with preceding multi-byte chars lands at the wrong byte boundary, producing invalid UTF-8 and potential panics.

## What Changes

- Fix `enter_char()` in `Input` to compute the correct byte offset from the char-based cursor using `char_indices()` before calling `String::insert()`
- Fix `enter_char()` in `InputMasked` with the same correction
- Enable Kitty keyboard protocol (`PushKeyboardEnhancementFlags`) during raw mode setup so modern terminals correctly pass CJK IME-composed characters
- Filter keyboard events to only process `KeyEventKind::Press` (required once keyboard enhancements are enabled, preventing double-fire from Release events)
- Fix horizontal scroll offset calculation to use visual width (via `unicode-width`) instead of char count
- Add `unicode-width` dependency to the crate

## Capabilities

### New Capabilities

- `cjk-text-input`: Text input widgets (`Input`, `InputMasked`) correctly handle multi-byte UTF-8 characters (CJK, emoji) at any cursor position, with correct rendering, scrolling, and deletion

### Modified Capabilities

- `password-input`: Fixes the same `enter_char` bug in `InputMasked` (shares the same byte-index bug)

## Impact

- `src/tui/popmsg/input.rs` — `enter_char()`, scroll offset in `render()`
- `src/tui/utils.rs` — `raw_mode::setup()` adding `PushKeyboardEnhancementFlags`
- `src/tui/app.rs` — filter `KeyEventKind::Press` in event loop
- `src/tui/key.rs` — possibly filter kind at conversion site
- `Cargo.toml` — new `unicode-width` dependency
