## 1. Dependencies

- [x] 1.1 Add `unicode-width = "0.2.0"` with `default-features = false` to `[dependencies]` in Cargo.toml

## 2. Core Bug Fix — Byte Offset in `Input`

- [x] 2.1 Add `byte_offset()` helper method to `Input` that converts `self.cursor` (char index) to byte offset using `char_indices()`
- [x] 2.2 Fix `enter_char()` to use `byte_offset()` before `String::insert()`, and remove `.min(self.buffer.len())` from the cursor update
- [x] 2.3 Fix `move_cursor_right()` to use `self.buffer.chars().count()` instead of `self.buffer.len()` as upper bound
- [x] 2.4 Fix `render()` to split the display string at a char-aligned byte boundary instead of using `self.cursor` directly

## 3. Core Bug Fix — Byte Offset in `InputMasked`

- [x] 3.1 Add the same `byte_offset()` helper to `InputMasked`
- [x] 3.2 Fix `enter_char()` in `InputMasked` (identical fix to Input)
- [x] 3.3 Fix `move_cursor_right()` in `InputMasked` (identical fix to Input)
- [x] 3.4 Fix `render()` in `InputMasked` for char-aligned string splitting

## 4. Scroll Offset with Wide Characters

- [x] 4.1 Fix scroll offset in `Input::render()` to use `unicode_width::UnicodeWidthStr::width()` instead of `self.cursor as u16`
- [x] 4.2 Fix scroll offset in `InputMasked::render()` to use width of the masked string prefix
- [x] 4.3 Fix `size()` method in both structs: replace `self.buffer.len() as u16` with visual width calculation

## 5. Keyboard Protocol & Event Filtering

- [x] 5.1 Add `PushKeyboardEnhancementFlags` with `REPORT_EVENT_TYPES` to `raw_mode::setup()`
- [x] 5.2 Add `PopKeyboardEnhancementFlags` to `raw_mode::restore()`
- [x] 5.3 Filter `Event::Key(key_event)` to only process `key_event.kind == KeyEventKind::Press` in `app.rs` event loop

## 6. Tests & Verification

- [x] 6.1 Write inline `#[test]` for `Input::enter_char` with CJK text (test insertion between multi-byte chars)
- [x] 6.2 Write inline `#[test]` for `Input::move_cursor_right` with multi-byte buffer
- [x] 6.3 Write inline `#[test]` for `InputMasked` with CJK text
- [x] 6.4 Run `cargo test` and verify all tests pass
- [x] 6.5 Run `cargo check` and verify no warnings
