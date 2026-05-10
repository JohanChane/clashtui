# Key Unification: Cross-Platform / Cross-Terminal Key Handling

Demotui's approach to ensuring consistent key events across terminal emulators and operating systems.

---

## Architecture

Two complementary mechanisms ensure that the same physical key press produces the same `Key` struct regardless of terminal emulator, OS, or whether CSI u is available:

| Layer | Mechanism | When |
|-------|-----------|------|
| Terminal protocol | CSI u (kitty keyboard protocol) probing | `term::setup()`, during `tui::init()` |
| Application code | `Key::from(KeyEvent)` normalization | Every key press, in the event loop |

### Terminal Lifecycle Module (`src/tui/term.rs`)

Mimics Yazi's `yazi-fm/src/term.rs` — full terminal suspend/resume for external processes:

| Function | Teardown order | Rebuild order |
|----------|---------------|---------------|
| `suspend()` | `\x1b[=0u` (disable CSI u) → `raw_mode::restore()` → show cursor | — |
| `resume()` | — | `raw_mode::setup()` → `\x1b[=5u` (re-enable CSI u) → panic hook |
| `setup()` | — | `raw_mode::setup()` → probe → `\x1b[=5u` → panic hook |
| `teardown()` | clear `CSI_U_ENABLED` → `\x1b[=0u` → `raw_mode::restore()` | — |
| `hold()` | `raw_mode::restore()` | `raw_mode::setup()` |

`raw_mode::setup()` / `raw_mode::restore()` (in `src/tui/utils.rs`) handle only raw mode + alternate screen — **no keyboard enhancement** (CSI u is managed exclusively by `term.rs`).

---

## Layer 1: CSI u Keyboard Enhancement

### Probing (`src/tui/term.rs:8-23`)

During `term::setup()`, after `raw_mode::setup()`:

1. Write the CSI u query `\x1b[?u` to stdout and flush
2. Sleep 50ms to allow the terminal to respond
3. Read raw bytes from stdin (up to 32 bytes)
4. Check if the response contains `\x1b[?0u` — this means the terminal responds to CSI u queries
5. If supported, write `\x1b[=5u` to enable `DISAMBIGUATE_ESCAPE_CODES | REPORT_ALTERNATE_KEYS`

The `=` command uses **absolute set** (not crossterm's stack-based Push/Pop). All kitty-protocol terminals support `=`, but some ignore the optional stack `>`/`<` commands. This avoids the bug where `PopKeyboardEnhancementFlags` (`\x1b[<1u`) is silently ignored, leaving CSI u escape sequences leaking into subprocess stdin.

### Teardown (`src/tui/term.rs:45-49`)

`teardown()`: clear `CSI_U_ENABLED` → `\x1b[=0u` → `raw_mode::restore()`.

`suspend()` (for external processes): `\x1b[=0u` → `raw_mode::restore()` → show cursor. Does NOT clear `CSI_U_ENABLED` — the flag persists so `resume()` knows to re-enable.

### What CSI u provides

When a terminal supports CSI u and these flags are enabled, it reports modifier-key combinations using unambiguous escape codes. For example:

| Physical press | Without CSI u | With CSI u |
|---------------|---------------|------------|
| `Ctrl+Shift+a` | May be reported as `Ctrl+a` (ambiguous) | Unambiguous escape code |
| `Alt+Enter` | May be indistinguishable from `Enter` | Unambiguous escape code |
| `Super+b` | Often not reported at all | Unambiguous escape code |

Terminals known to support CSI u: kitty, WezTerm, Ghostty, foot, Konsole (newer versions), iTerm2 (with preferences enabled), Windows Terminal (1.20+).

Terminals without CSI u support still work — the application-level normalization (Layer 2) handles the most common cross-platform discrepancies.

---

## Layer 2: Application-Level Normalization

### Key Struct (`src/tui/key.rs:5-12`)

```rust
pub struct Key {
    pub code:   KeyCode,   // crossterm's KeyCode enum
    pub shift:  bool,
    pub ctrl:   bool,
    pub alt:    bool,
    pub super_: bool,
}
```

Modifiers are **flat booleans**, not a bitmask. This makes equality comparison (`PartialEq`, `Hash`) straightforward.

### `From<KeyEvent>` Normalization (`src/tui/key.rs:29-50`)

The conversion from crossterm's raw `KeyEvent` applies three fixups:

#### (a) Alphabetic shift inference

```rust
(KeyCode::Char(c), _) if c.is_ascii_uppercase() => shift = true
```

Shift is derived from the character's case (`'A'` → shift=true, `'a'` → shift=false), **not** from `KeyModifiers::SHIFT`. This handles the cross-platform disparity where:

- **Unix**: `Shift+a` may produce `Char('A')` with **no** SHIFT modifier
- **Windows**: `Shift+a` may produce `Char('A')` **with** SHIFT modifier

Both produce `Key { code: Char('A'), shift: true }`.

#### (b) Non-alphabetic shift stripping

```rust
(KeyCode::Char(c), m) if !c.is_ascii_alphabetic() && m.contains(SHIFT) => shift = false
```

Non-alphabetic characters like `~`, `!`, `@`, `#` already represent their shifted form. On Windows, crossterm reports these with a SHIFT modifier; on Unix, it does not. Stripping the SHIFT flag ensures both platforms produce identical `Key` values:

- `Shift+~` on Windows: `KeyEvent { Char('~'), SHIFT }` → `Key { Char('~'), shift: false }`
- `Shift+~` on Unix: `KeyEvent { Char('~'), NONE }` → `Key { Char('~'), shift: false }`

#### (c) BackTab special case

```rust
(KeyCode::BackTab, _) => shift = false
```

`BackTab` (`Shift+Tab`) always has `shift = false` because the `BackTab` key code itself implies shift.

---

## Key Parsing (`FromStr`)

Human-readable key strings (used in YAML keymaps and `mod_agent!` macros) support two forms:

| Form | Example | Result |
|------|---------|--------|
| Bare char | `"a"`, `"A"`, `"/"` | `Key { code: Char('a'), shift: false }` / `Key { code: Char('A'), shift: true }` |
| Angle bracket | `"<C-S-x>"`, `"<A-b>"`, `"<Enter>"` | `Key { code: Char('x'), ctrl: true, shift: true }` |

Modifier prefixes (case-insensitive): `S-` (shift), `C-` (ctrl), `A-` (alt), `D-` (super/desktop).

Named keys: `Space`, `Backspace`, `Enter`, `Left`, `Right`, `Up`, `Down`, `Home`, `End`, `PageUp`, `PageDown`, `Tab`, `BackTab`, `Delete`, `Insert`, `Esc`.

## Key Display (`Display`)

Renders keys back to human-readable form:

- `Key { code: Char('a'), shift: false }` → `"a"`
- `Key { code: Char('A'), shift: true }` → `"A"`
- `Key { code: Char(' '), shift: false }` → `"<Space>"`
- `Key { code: Char('x'), ctrl: true, shift: true }` → `"<C-S-x>"`
- `Key { code: Enter }` → `"<Enter>"`

Modifier ordering: `D-` → `C-` → `A-` → `S-`.

---

## Comparison: demotui vs Yazi

### Identical

| Aspect | Both |
|--------|------|
| Key struct | `{ code: KeyCode, shift, ctrl, alt, super_ }` — flat booleans |
| Shift inference | Derived from `c.is_ascii_uppercase()`, not from KeyModifiers |
| Non-alpha shift stripping | SHIFT stripped from non-alphabetic chars (`~`, `!`, etc.) |
| CSI u probing | Query `\x1b[?u` and enable DISAMBIGUATE + ALTERNATE_KEYS flags |
| Key parsing | Angle bracket syntax: `<C-S-x>`, `<A-b>`, `<D-Enter>` |
| Key display | Bare chars for plain keys, `<C-S-x>` for modified keys |
| BackTab handling | Always `shift = false` |

### Differences

| Aspect | demotui | Yazi |
|--------|---------|------|
| Terminal detection | CSI u probing only | Full emulator detection: 26 terminal brands via CSI DA1 + env vars (`brand.rs`) |
| Tmux support | None | Tmux passthrough mode detection and escape wrapping (`mux.rs`) |
| CSI u probing method | Simple: write query, sleep 50ms, read stdin | Integrated into multi-query `Emulator::read_until_da1()` alongside cursor shape, blink, and device attribute queries |
| `Key::plain()` | `pub fn plain(&self) -> Option<char>` — returns char only if no ctrl/alt/super modifiers | No equivalent |
| Named F-keys in Display | Only `F(1)` via the catch-all `_ => "Unknown"` | Explicitly names `F(1)` through `F(19)` |
| Fzf implementation | External `fzf` binary via subprocess | External `fzf` binary via Lua plugin |
| Event acquisition | `crossterm::EventStream` via `futures_lite::StreamExt` | Custom `Signals` task with `tokio::sync::mpsc::unbounded_channel` |
| Resize handling | Atomic flag (`RESIZE.store(true, ...)`) checked at top of next frame | Direct repaint on resize event |
| Key routing | Six-layer dispatch: PopUp → GlobalChord → Help → Chord → Tab → Global | Layer system: Which → Cmp → Help → Confirm → Input → Pick → Spot → Tasks → Mgr |

### Rationale for demotui's simpler approach

Demotui targets users who run clash-based proxy management, typically on desktop Linux/macOS with a single terminal emulator. Full terminal brand detection (26 brands) and tmux passthrough are unnecessary complexity for this use case. The two-layer approach (CSI u probing + application normalization) covers the most common cross-platform key discrepancies without the maintenance burden of a full emulator detection system.

---

## Files

| File | Role |
|------|------|
| `src/tui/term.rs` | Terminal lifecycle: CSI u probe/enable/disable, `setup`/`teardown`/`hold`/`suspend`/`resume` |
| `src/tui/utils.rs` | `raw_mode::setup/restore`: raw mode + alternate screen (no keyboard enhancement) |
| `src/tui.rs` | Module graph, `TuiWidget` trait, `EXT_PROC` flag, delegates to `term` |
| `src/tui/key.rs` | `Key` struct, `From<KeyEvent>`, `FromStr`, `Display`, `plain()` |
| `src/tui/app.rs` | Event loop: `KeyEvent` → `Key::from()` → six-layer dispatch |
| `src/tui/widget/fzffind.rs` | `run_fzf()`: external fzf subprocess wrapper |

## External Fzf Integration

Demotui delegates fuzzy finding to the external `fzf` binary (just like Yazi).

### Implementation

`src/tui/widget/fzffind.rs` exports a single function:

```rust
pub fn run_fzf(items: &[String], prompt: &str) -> Option<usize>
```

Flow:
1. `crate::tui::EXT_PROC` atomic flag is set → prevents event loop rendering
2. `crate::tui::suspend_terminal()` → `term::suspend()`:
   - `\x1b[=0u` disables CSI u (absolute set, works on all terminals)
   - `raw_mode::restore()` exits raw mode + leaves alternate screen + shows cursor
3. `fzf` is spawned with `--delimiter='\t' --with-nth=2`:
   - Input lines: `{index}\t{display_name}` — fzf displays only `display_name`
   - Output: the full line — first tab-delimited field is the selected index
4. On completion: `crate::tui::resume_terminal()` → `term::resume()`:
   - `raw_mode::setup()` re-enters raw mode + alternate screen
   - `\x1b[=5u` re-enables CSI u (only if `CSI_U_ENABLED` flag was set)
   - Resets panic hook
5. `EXT_PROC` cleared, `FULL_RENDER` notified for full terminal redraw

### Call sites

| Tab | File | Trigger |
|-----|------|---------|
| Profile | `src/tui/tab/files/profile.rs:272` | `f` key → `Action::FzfFind` |
| Template | `src/tui/tab/files/template.rs:262` | `f` key → `Action::FzfFind` |
| Proxies | `src/tui/tab/proxies/handlers.rs:22` | `f` key → `Key::FzfFind` |

All call sites use `tokio::task::spawn_blocking` to offload the synchronous fzf subprocess from the async runtime. The event loop is prevented from rendering while the external process runs via `crate::tui::EXT_PROC: AtomicBool`.

### Comparison: demotui vs Yazi fzf approach

| Aspect | demotui | Yazi |
|--------|---------|------|
| Invocation | `std::process::Command::new("fzf")` | `Command("fzf")` via Lua plugin |
| Terminal mode | Full suspend/resume: disable CSI u, leave alt screen, exit raw mode; rebuild on resume | `Term::stop()` → fzf → `Term::start()`: full teardown/rebuild |
| Input format | `{idx}\t{name}` with `--with-nth=2` | File paths, one per line |
| Output parsing | Parse `\t`-delimited first field as index | Parse selected file path directly |
| Selection mapping | Index mapped to content item position | Path matched back to file entry |
| Event loop | Skipped via `EXT_PROC` atomic flag during fzf | Plugin blocks Lua task; event loop continues |
| Dependency | `fzf` must be in `$PATH` | `fzf` listed as system dependency in packaging |

## Test Coverage

`src/tui/key.rs` contains 10 unit tests covering:
- Plain chars, uppercase chars, ctrl-modified keys
- Non-alpha shift stripping on both Windows-style and Unix-style inputs
- Shift-digit normalization (`Shift+1` → `!`)
- BackTab, non-char keys with shift
- All normalization produces identical `Key` values across simulated platform differences
