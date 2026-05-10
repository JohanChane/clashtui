## Context

The `Input` and `InputMasked` widgets in `src/tui/popmsg/input.rs` use a `cursor: usize` field that tracks **character position** — incremented/decremented by 1 per character. However, multiple methods treat this cursor as a **byte index**, which is only equivalent for ASCII text (1 char = 1 byte). For multi-byte UTF-8 (CJK, emoji: 1 char = 2-4 bytes), this causes incorrect behavior.

The event pipeline (`app.rs` → `key.rs` → `input.rs`) processes `KeyCode::Char(c)` events from crossterm, which correctly decode raw terminal bytes into Unicode `char` values. The problem is purely in how the consumer widgets use the cursor.

Yazi (reference implementation) avoids this by consistently using `char_indices()` to convert char positions to byte offsets, and by tracking cursor as a char index throughout. It also enables the Kitty keyboard protocol for better IME passthrough on modern terminals.

## Goals / Non-Goals

**Goals:**
- Fix `enter_char()` to compute the correct byte offset before calling `String::insert()`
- Fix `move_cursor_right()` to use char count, not byte count, as the upper bound
- Fix `render()` methods to split strings at byte boundaries aligned with char boundaries
- Enable Kitty keyboard protocol (`REPORT_EVENT_TYPES`) for better IME support on modern terminals
- Filter `KeyEventKind::Press` to avoid double-processing if keyboard enhancements become active
- Add `unicode-width` for correct horizontal scroll offset with wide characters

**Non-Goals:**
- IME preedit (composition) overlay — requires significant new infrastructure, out of scope
- Vi-like input modes (Normal/Insert/Replace) — existing behavior is Insert-only, unchanged
- Tab-completion or other input enhancements

## Decisions

### 1. Convert char index to byte index via `char_indices()`

The cursor is semantically a char index. Before any byte-level `String` operation (`insert`, `split_off`), convert using:

```rust
fn byte_offset(&self) -> usize {
    self.buffer.char_indices()
        .nth(self.cursor)
        .map(|(i, _)| i)
        .unwrap_or(self.buffer.len())
}
```

**Rationale**: `char_indices()` is O(n) in the worst case but correct. Input buffers are short (user-typed text, typically <100 chars), so performance is not a concern. This is the same approach Yazi uses (see `snap.idx()`).

### 2. Fix `move_cursor_right` upper bound

Change from `.min(self.buffer.len())` (byte count) to `.min(self.buffer.chars().count())` (char count).

**Rationale**: `buffer.len()` returns byte length, which for Chinese text "你好" is 6, but the valid cursor range is 0..=2. Using `chars().count()` is O(n) but correct. Alternative considered: tracking char count separately in an additional field — rejected as premature optimization.

### 3. Enable Kitty keyboard protocol

Add `PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)` in `raw_mode::setup()` and filter for `KeyEventKind::Press` in the event loop.

**Rationale**: Modern terminals (kitty, alacritty, wezterm, foot, ghostty) use this protocol to:
- Distinguish Press/Release/Repeat events (preventing double-fire from Release)
- Pass through IME-composed characters correctly

**Risk mitigation**: `PushKeyboardEnhancementFlags` is a no-op on terminals that don't support the Kitty protocol. It degrades gracefully — older terminals simply ignore the escape sequence.

### 4. Filter `KeyEventKind::Press` at event dispatch

Add a guard in `app.rs`:

```rust
Event::Key(key_event) if key_event.kind == KeyEventKind::Press => { ... }
```

Without this, enabling keyboard enhancements would cause Release events to double-insert characters.

**Alternative considered**: Filtering at `Key::from()`. Rejected because the Key struct is also used for keymap configuration where distinguishing Press/Release could be useful in the future.

### 5. Add `unicode-width` for scroll offset

Change scroll offset from `self.cursor as u16` to use `UnicodeWidthStr::width()` on the text before the cursor.

**Rationale**: Chinese characters are 2 cells wide. Without this, the horizontal scroll doesn't track correctly when wide characters are before the cursor — the viewport scrolls too little.

### 6. Fix `render()` for cursor-based string splitting

Replace `before.split_off(self.cursor)` with byte-aligned splitting using `char_indices()`.

**Rationale**: `String::split_off()` splits at byte offset. When `cursor` is a char index and the buffer contains multi-byte text, this panics with "not at a valid UTF-8 character boundary".

## Risks / Trade-offs

- **Kitty protocol on legacy terminals**: `PushKeyboardEnhancementFlags` is defined to be quietly ignored by terminals that don't understand it — no risk.
- **`KeyEventKind` compatibility**: `KeyEventKind` was stabilized in crossterm 0.27. Current Cargo.toml uses `crossterm = "*"` which resolves to >= 0.28 — fully supported.
- **Performance of O(n) char operations**: Input buffers are user-typed text (small n). No measurable impact.
- **`InputMasked` regression**: The masked widget shares the same `enter_char`/`move_cursor_right` bugs. Fix must cover both structs. The masked widget's `render()` also has the `split_off` bug.
