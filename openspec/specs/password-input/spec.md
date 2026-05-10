# password-input Specification

## Purpose
TBD - created by archiving change migrate-clashsrvctl. Update Purpose after archive.
## Requirements
### Requirement: Masked text rendering
The password input widget SHALL render user-typed characters as a masking character (`●` or `*`) instead of the actual character, while storing the real characters in its internal buffer.

#### Scenario: User types a password
- **WHEN** user types the character 'a' into the password input
- **THEN** a mask character is displayed at the cursor position instead of 'a', and the real character 'a' is appended to the internal buffer

#### Scenario: Cursor movement with masked display
- **WHEN** user moves cursor left or right
- **THEN** the cursor visual indicator moves, but all displayed characters remain masked

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

### Requirement: Confirm via Enter, cancel via Esc
The widget SHALL send the password string via the oneshot channel when Enter is pressed (Route::Send), and drop the channel (cancelling) when Esc is pressed (Route::Drop).

#### Scenario: User confirms password entry
- **WHEN** user types "secret" and presses Enter
- **THEN** the oneshot receiver resolves with the string "secret"

#### Scenario: User cancels password entry
- **WHEN** user presses Esc
- **THEN** the oneshot receiver gets a `RecvError` (channel closed without value)

### Requirement: Implements Msg trait
The widget SHALL implement `Msg<Result = String>` so it can be used with the existing `MsgBuilder` and `PAIR` channel infrastructure.

#### Scenario: Building and sending a password input popup
- **WHEN** code calls `InputMasked::new().with_title("Sudo Password".to_owned()).build_and_send().await`
- **THEN** a masked input popup appears, the caller awaits on the receiver, and the result is delivered on Enter or dropped on Esc

### Requirement: Visual distinction from regular input
The password input SHALL be visually distinguishable from the regular `Input` widget, displaying a prompt that indicates it is a password field (e.g., "Enter sudo password" title).

#### Scenario: Password popup appearance
- **WHEN** the password popup is displayed
- **THEN** the popup title indicates "Sudo Password" or similar, and the input field shows masked characters

