# cjk-text-input Specification

## Purpose
TBD - created by archiving change fix-cjk-input-support. Update Purpose after archive.
## Requirements
### Requirement: Multi-byte character insertion at any cursor position
The `Input` widget SHALL correctly insert multi-byte UTF-8 characters (CJK, emoji, accented Latin) at any cursor position, including positions before existing multi-byte characters.

#### Scenario: Insert CJK character after existing CJK text
- **WHEN** buffer contains "你好" (2 chars, 6 bytes) and cursor is at position 1 (between 你 and 好)
- **THEN** pressing Char 'x' inserts 'x' between the two characters producing "你x好" without UTF-8 corruption or panic

#### Scenario: Insert CJK character at beginning
- **WHEN** buffer contains "你好" and cursor is at position 0
- **THEN** pressing Char '中' inserts at position 0 producing "中你好"

#### Scenario: Insert CJK character at end
- **WHEN** buffer contains "" (empty) and user types "你好世界"
- **THEN** buffer correctly contains "你好世界" with cursor at position 4 (4 chars)

### Requirement: Cursor movement respects character boundaries
The `Input` widget SHALL track cursor position as a character index and constrain movement to valid character boundary positions.

#### Scenario: Move cursor right after multi-byte text
- **WHEN** buffer is "你好" (2 chars) and cursor is at position 1
- **THEN** pressing Right moves cursor to position 2 (not to byte offset 2)

#### Scenario: Move cursor right at end
- **WHEN** buffer is "你好" and cursor is at position 2 (end)
- **THEN** pressing Right keeps cursor at position 2 (does not overflow)

### Requirement: Correct rendering with multi-byte characters
The `Input` widget SHALL render multi-byte text correctly, splitting the display at character boundaries for cursor placement.

#### Scenario: Render CJK text with cursor mid-string
- **WHEN** buffer is "你好世界" and cursor is at position 2
- **THEN** the rendered output shows the cursor character (inverted/highlighted) at the correct visual position corresponding to the character after the first two

#### Scenario: Render does not panic on string split
- **WHEN** any render with a non-ASCII buffer and non-zero cursor position is triggered
- **THEN** no panic occurs from `split_off` on a non-char boundary

### Requirement: Correct scroll offset with wide characters
The horizontal scroll SHALL use visual width (accounting for wide characters being 2 cells) rather than raw char count.

#### Scenario: Scroll accounts for wide chars
- **WHEN** buffer begins with "你好" (4 cells wide) and cursor is at position 2
- **THEN** scroll offset accounts for the 4-cell visual width, not 2

### Requirement: KeyEvent kind filtering
The event loop SHALL only process `KeyEventKind::Press` events, ignoring `Release` and `Repeat` events.

#### Scenario: Release event is ignored
- **WHEN** terminal sends a `KeyEvent { kind: KeyEventKind::Release, code: Char('x') }`
- **THEN** the event is discarded and no character is inserted

#### Scenario: Press event is processed normally
- **WHEN** terminal sends a `KeyEvent { kind: KeyEventKind::Press, code: Char('你') }`
- **THEN** the character '你' is inserted into the active input

### Requirement: Kitty keyboard protocol enabled
The terminal setup SHALL push `KeyboardEnhancementFlags::REPORT_EVENT_TYPES` to enable event-type disambiguation on supporting terminals.

#### Scenario: Protocol degrades gracefully
- **WHEN** running on a terminal that does not support the Kitty keyboard protocol
- **THEN** the application starts and operates normally (the enhancement escape sequence is silently ignored)

#### Scenario: Protocol active on modern terminal
- **WHEN** running on kitty/alacritty/wezterm/foot/ghostty
- **THEN** CJK IME-composed characters are delivered as `KeyCode::Char(c)` with `KeyEventKind::Press`

