# Modifier Keys

demotui's key handling departs from raw `crossterm::KeyEvent` and adopts a flat, modifier-aware `Key` struct inspired by yazi. This document explains the design, usage conventions, and signal handling integration.

## Key Struct (`src/tui/key.rs`)

```rust
pub struct Key {
    pub code:   KeyCode,   // from crossterm: Char, Enter, Up, F1, etc.
    pub shift:  bool,
    pub ctrl:   bool,
    pub alt:    bool,
    pub super_: bool,      // a.k.a. "Windows key" / "Command key"
}
```

**Design rationale: flat booleans, not bitmask.** Each modifier is an independent field, enabling simple destructuring and pattern matching:

```rust
match key {
    Key { code: KeyCode::Char('c'), ctrl: true, alt: false, .. } => quit(),
    Key { code: KeyCode::Char('q'), ctrl: false, .. } => type_q(),
    _ => (),
}
```

### Construction from crossterm events (`From<KeyEvent>`)

```rust
impl From<KeyEvent> for Key {
    fn from(ev: KeyEvent) -> Self { ... }
}
```

Conversion normalizes shift detection cross-platform:

| Scenario | Shift |
|---|---|
| `Char('A')` (`is_ascii_uppercase()`) | `true` (inferred from character) |
| `BackTab` | always `false` (crossterm reports shift spuriously) |
| Non-char keys (Up, Down, F1, etc.) | read from `KeyModifiers::SHIFT` |

Ctrl, Alt, Super come directly from `KeyModifiers`. This mirrors yazi's `From<Key>` impl but uses `is_ascii_uppercase()` for char-case inference, which works on all platforms without reliance on terminal raw-mode details.

### String Parsing (`FromStr`)

Keys can be parsed from strings using the bracket syntax:

```
<C-x>      Ctrl + x
<A-b>      Alt + b
<S-Up>     Shift + Up
<D-h>      Super (Windows/Command) + h
<C-A-x>    Ctrl + Alt + x
```

Modifier prefixes (case-insensitive):

| Prefix | Modifier |
|---|---|
| `S-` | Shift |
| `C-` | Ctrl |
| `A-` | Alt |
| `D-` | Super |

Reverse order (in `Display`): `D` → `C` → `A` → `S`, followed by the key name.

**Without brackets**, the string is treated as a plain character:

```
"j"  → Key { code: Char('j'), shift: false }
"Q"  → Key { code: Char('Q'), shift: true }
```

The Shift flag is automatically set for uppercase ASCII characters.

**Named keys** (inside brackets):

`space`, `backspace`, `enter`, `left`, `right`, `up`, `down`, `home`, `end`, `pageup`, `pagedown`, `tab`, `backtab`, `delete`, `insert`, `esc`

Example: `<C-enter>` → `Key { code: Enter, ctrl: true }`.

### Display (`Display` impl)

Keys display in human-readable form:

| Key | Display |
|---|---|
| `Key { code: Char('c'), ctrl: true }` | `<C-c>` |
| `Key { code: Char('q'), ctrl: false, .. }` | `q` |
| `Key { code: Char(' '), .. }` | `<Space>` |
| `Key { code: Backspace, alt: true }` | `<A-Backspace>` |
| `Key { code: Char('A'), shift: true }` | `A` |

### `plain()` Helper

```rust
pub fn plain(&self) -> Option<char>
```

Returns `Some(c)` only when the key is a plain printable character (no ctrl, alt, or super modifiers). Used for text input filtering (e.g., search queries, inline editing).

## Tab Key Bindings (`mod_agent!`)

Each tab defines key bindings via `mod_agent!` in `src/tui/tab/mod.rs`. The macro accepts two forms:

### Form 1: `KeyCode` tuples (legacy, single keys only)

```rust
mod_agent!(
    Key,
    [
        ([KeyCode::Char('j')],    Key::MoveDown, ""),
        ([KeyCode::Enter],        Key::Select,   ""),
    ]
);
```

The `quick_map` helper auto-detects shift from uppercase ASCII characters. Generated `Key` entries always have `ctrl: false, alt: false, super_: false`.

### Form 2: `key("<...>")` syntax (recommended for modifier keys)

```rust
mod_agent!(
    Key,
    [
        (key("<C-c>"), Key::Close, "Close connection"),
        (key("<S-Tab>"), Key::Previous, "Previous tab"),
    ]
);
```

The `key("<...>")` muncher calls `Key::from_str` at init time. This is the **only way** to define bindings with Ctrl/Alt/Super modifiers in `mod_agent!`.

### Agent Lookup

Bindings populate a `HashMap<crate::tui::Key, TabKey>` (the "agent"). At runtime, tab key dispatch works by looking up `crate::tui::Key` → tab-local `Key` enum:

```rust
impl TryFrom<&crate::tui::Key> for TabKey {
    fn try_from(ev: &crate::tui::Key) -> Result<Self, Self::Error> {
        agent().get(ev).map(|k| *k).ok_or(())
    }
}
```

## Chord Handler (`src/tui/widget/chord.rs`)

Multi-key shortcuts (e.g., `g g` for "go to top") are defined as `KeyCode` tuples in `mod_agent!`:

```rust
([KeyCode::Char('g'), KeyCode::Char('g')], Key::GoTop, "Go to top"),
([KeyCode::Char('s'), KeyCode::Char('s')], Key::ToggleSort, "Toggle sort"),
([KeyCode::Char('a'), KeyCode::Char('f')], Key::CollapseAll, "Collapse all"),
```

Chords are **always plain keys** (no modifiers). The ChordHandler processes them progressively:

1. **First keypress**: filters `all_shortcuts()` to candidates whose prefix matches
2. **Subsequent keypresses**: narrows candidates; ESC (without modifiers) or Ctrl-C cancels the chord
3. **Exact match**: dispatches the action and resets

## Signal Handling (`src/tui/signals.rs`)

On Unix systems, a spawned tokio task manages OS signals via `signal_hook_tokio`:

| Signal | Action |
|---|---|
| SIGINT | **Ignored** — Ctrl-C is a keyboard event in raw mode |
| SIGQUIT, SIGHUP, SIGTERM | Sets `QUIT` atomic → graceful shutdown |
| SIGTSTP (Ctrl-Z) | Calls `hold(true)` (restore terminal) → `kill(0, SIGTSTP)` (self-stop) → on resume: `hold(false)` (re-enter raw mode) + `FULL_RENDER.notify_one()` |
| SIGCONT | (registered but not handled — resume is driven by SIGTSTP path) |

The main event loop checks `QUIT.load(Relaxed)` each frame. SIGINT is explicitly ignored at the OS level because raw mode disables `ISIG`, meaning Ctrl-C arrives as a normal `KeyEvent` rather than a signal.

Terminal resize events are detected via crossterm's `EventStream` and propagated through the existing resize handling path.

## Usage in Tabs

### Defining a tab's key bindings

```rust
mod_agent!(
    Key,
    [
        ([KeyCode::Up],           Key::MoveUp,      ""),
        ([KeyCode::Down],         Key::MoveDown,    ""),
        ([KeyCode::Char('j')],    Key::MoveDown,    ""),
        ([KeyCode::Char('k')],    Key::MoveUp,      ""),
        (key("<C-c>"),            Key::Close,       "Close"),
        (key("<S-Tab>"),          Key::Previous,    "Previous"),
    ]
);
```

### Dispatching in `handle_key_event`

```rust
fn handle_key_event(&mut self, kv: &crate::tui::Key) {
    if let Ok(act) = Self::Key::try_from(kv) {
        match act {
            Self::Key::Close => { /* ... */ }
            Self::Key::MoveUp => { /* ... */ }
            _ => {},
        }
    }
}
```

## Key Naming Convention

- **`crate::tui::Key`** — the global key type (in `src/tui/key.rs`)
- **`Key` (tab-local)** — per-tab enum of semantic actions (e.g., `proxies::Key`, `connections::Key`)
- In `mod_agent!` and tab sources, always use `crate::tui::Key` (fully qualified) to disambiguate from the local `Key` enum
- The `dev` module re-exports `crate::tui::Key as TuiKey` for use in `use super::dev::*` patterns
