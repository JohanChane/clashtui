## MODIFIED Requirements

### Requirement: Common input editing behavior
The widget SHALL support standard text editing operations: character insertion at cursor, backspace (delete before cursor), Delete (delete at cursor), left/right cursor movement, Home, End. All operations SHALL correctly handle multi-byte UTF-8 characters at any cursor position by converting the char-based cursor index to a byte offset before performing byte-level String operations.

#### Scenario: Backspace in password field
- **WHEN** user presses Backspace with cursor at position 3
- **THEN** the character at position 2 is removed from the buffer and the displayed mask count decreases by one

#### Scenario: Cursor bounds
- **WHEN** cursor is at position 0 and user presses Left
- **THEN** cursor remains at position 0 (does not wrap or panic)

#### Scenario: Insert CJK character into password field
- **WHEN** buffer contains "ab" and cursor is at position 1, and user types the Chinese character '你'
- **THEN** buffer contains "a你b", cursor is at position 2, and displayed mask shows 3 mask characters

#### Scenario: Move cursor right with multi-byte buffer
- **WHEN** buffer contains "你好" (2 chars, 6 bytes) and cursor is at position 1
- **THEN** pressing Right moves cursor to position 2, not to byte offset 2
